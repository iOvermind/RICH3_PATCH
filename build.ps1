# Rich Patch Series — Windows PowerShell 打包腳本
#
# 用法：
#   .\build.ps1              # 打包
#   .\build.ps1 -Sign        # 打包並嘗試數位簽章
#   .\build.ps1 -SkipDeps    # 跳過套件安裝檢查 (較快)

[CmdletBinding()]
param(
    [switch]$Sign,
    [switch]$SkipDeps
)

$ErrorActionPreference = 'Stop'

$AppName  = 'rich3_patch'
$AppLabel = 'Richman 3 Patch'

# 不管從哪裡呼叫，一律以腳本所在目錄為工作目錄
Set-Location -Path $PSScriptRoot

Write-Host '==================================' -ForegroundColor Cyan
Write-Host "  [+] $AppLabel Builder Pro"       -ForegroundColor Cyan
Write-Host '  [+] Author: Overmind'            -ForegroundColor Cyan
Write-Host '==================================' -ForegroundColor Cyan

# ---------------------------------------------------------------
# 1. 環境檢查
# ---------------------------------------------------------------
$python = Get-Command python -ErrorAction SilentlyContinue
if (-not $python) {
    Write-Host '[ERROR] 找不到 python，請先安裝 Python 並加入 PATH。' -ForegroundColor Red
    exit 1
}
Write-Host "[*] Python: $(python --version)"

if (-not $SkipDeps) {
    Write-Host '[*] Checking dependencies...'
    python -m pip install --quiet --disable-pip-version-check -r requirements.txt
    if ($LASTEXITCODE -ne 0) {
        Write-Host '[ERROR] 套件安裝失敗，檢查一下 requirements.txt。' -ForegroundColor Red
        exit 1
    }
}

# ---------------------------------------------------------------
# 2. 清理舊檔案
# ---------------------------------------------------------------
Write-Host '[*] Cleaning old files...'
foreach ($dir in @('build', 'dist')) {
    if (Test-Path $dir) { Remove-Item -Recurse -Force $dir }
}

# ---------------------------------------------------------------
# 3. 打包 EXE
#    一律走 .spec 檔，資源清單與瘦身用的 EXCLUDES 才會生效
# ---------------------------------------------------------------
Write-Host '[*] Building EXE with PyInstaller...'
python -m PyInstaller --clean --noconfirm "$AppName.spec"
if ($LASTEXITCODE -ne 0) {
    Write-Host '[ERROR] PyInstaller failed! 屁啦，檢查一下 Python 套件。' -ForegroundColor Red
    exit 1
}

$exePath = Join-Path 'dist' "$AppName.exe"
if (-not (Test-Path $exePath)) {
    Write-Host '[ERROR] 打包跑完了卻找不到 EXE，怪。' -ForegroundColor Red
    exit 1
}

# ---------------------------------------------------------------
# 4. 數位簽章 (可選，預設跳過)
# ---------------------------------------------------------------
if ($Sign) {
    Write-Host '[*] Signing the executable...'

    if (-not (Test-Path 'Overmind.pfx')) {
        Write-Host '[*] Generating Certificate...'
        $cert = New-SelfSignedCertificate -Type CodeSigningCert -Subject 'CN=Overmind' `
            -KeyExportPolicy Exportable -KeySpec Signature -KeyLength 2048 `
            -KeyAlgorithm RSA -HashAlgorithm SHA256 -NotAfter (Get-Date).AddYears(10) `
            -CertStoreLocation 'Cert:\CurrentUser\My'
        $pfxPwd = ConvertTo-SecureString -String 'overmind' -Force -AsPlainText
        Export-PfxCertificate -Cert $cert -FilePath '.\Overmind.pfx' -Password $pfxPwd | Out-Null
        Write-Host '[OK] Overmind.pfx created.'
    }

    $signtool = Get-ChildItem 'C:\Program Files (x86)\Windows Kits\10\bin\*\x64\signtool.exe' -ErrorAction SilentlyContinue |
        Sort-Object FullName -Descending | Select-Object -First 1

    if ($signtool) {
        & $signtool.FullName sign /f 'Overmind.pfx' /p 'overmind' /fd SHA256 `
            /t http://timestamp.digicert.com /d $AppLabel $exePath
        if ($LASTEXITCODE -ne 0) {
            Write-Host '[WARN] 簽章過程報錯，請檢查憑證或網路連線。' -ForegroundColor Yellow
        }
    } else {
        Write-Host '[WARN] 找不到 signtool.exe，跳過簽章。' -ForegroundColor Yellow
    }
}

# ---------------------------------------------------------------
# 5. 回報成品體積，隨時盯著不要肥起來
# ---------------------------------------------------------------
$sizeMB = [math]::Round((Get-Item $exePath).Length / 1MB, 2)
Write-Host ''
Write-Host "[SIZE] $AppName.exe = $sizeMB MB"
Write-Host "[DONE] 完工！請到 dist 資料夾查看 $AppName.exe" -ForegroundColor Green
