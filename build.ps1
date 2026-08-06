# 大富翁2 Patch —— 建置與發佈打包
#
# 用法：
#   .\build.ps1                # 建置並收進 release\
#   .\build.ps1 -Sign          # 另外用自簽憑證簽章
#   .\build.ps1 -SkipInstall   # 跳過 npm ci（相依沒動過時較快）
#
# 產物命名依 docs/rules/RELEASE_RULES.md §2.1，中括號不可省略。

[CmdletBinding()]
param(
    [switch]$Sign,
    [switch]$SkipInstall
)

$ErrorActionPreference = 'Stop'
Set-Location -Path $PSScriptRoot

$ProjectName = 'RICH3_PATCH'
$AppLabel = '大富翁3 Patch'

function Step($text) { Write-Host "[*] $text" -ForegroundColor Cyan }
function Fail($text) { Write-Host "[ERROR] $text" -ForegroundColor Red; exit 1 }

# PowerShell 5.1 會把原生指令寫到 stderr 的每一行包成 ErrorRecord；配上
# ErrorActionPreference='Stop'，即使指令成功（npm、cargo 都會把進度訊息寫到 stderr）
# 也會被當成失敗而中止。原生指令一律走這裡，成敗只看離開碼。
function Invoke-Native {
    param([scriptblock]$Command, [string]$What)

    $previous = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    & $Command
    $code = $LASTEXITCODE
    $ErrorActionPreference = $previous

    if ($code -ne 0) { Fail "$What 失敗（離開碼 $code）" }
}

Write-Host '==================================' -ForegroundColor Cyan
Write-Host "  [+] $AppLabel Builder"           -ForegroundColor Cyan
Write-Host '  [+] Author: Overmind'            -ForegroundColor Cyan
Write-Host '==================================' -ForegroundColor Cyan

# ---------------------------------------------------------------
# 1. 版本號一致性 —— 發佈門檻，不一致就不准往下走
#
#    單一來源是 package.json；其餘位置必須跟它一致。這裡只讀不寫：
#    版本號一律手動改，這道檢查負責攔住漏改的那一處。
# ---------------------------------------------------------------
Step '檢查版本號一致性'

$version = (Get-Content 'package.json' -Raw -Encoding UTF8 | ConvertFrom-Json).version
if (-not $version) { Fail 'package.json 讀不到 version' }

$tauriVersion = (Get-Content 'src-tauri\tauri.conf.json' -Raw -Encoding UTF8 | ConvertFrom-Json).version
$cargoVersion = (Select-String 'src-tauri\Cargo.toml' -Pattern '^version\s*=\s*"(.+)"' |
    Select-Object -First 1).Matches.Groups[1].Value

$sources = [ordered]@{
    'package.json (單一來源)'     = $version
    'src-tauri/tauri.conf.json'  = $tauriVersion
    'src-tauri/Cargo.toml'       = $cargoVersion
}
$sources.GetEnumerator() | ForEach-Object { Write-Host ('      {0,-28} {1}' -f $_.Key, $_.Value) }

$mismatched = $sources.Values | Where-Object { $_ -ne $version }
if ($mismatched) { Fail "版本號不一致，發佈前必須先對齊（見 docs/rules/VERSION_RULES.md §2.3）" }
Write-Host "      → v$version" -ForegroundColor Green

# ---------------------------------------------------------------
# 2. 建置
# ---------------------------------------------------------------
if (-not $SkipInstall) {
    Step '安裝前端相依 (npm ci)'
    Invoke-Native { npm ci } 'npm ci'
}

Step '建置 (tauri build)'
Invoke-Native { npm run tauri build } 'tauri build'

# ---------------------------------------------------------------
# 3. 收進 release\ 並依規範命名
# ---------------------------------------------------------------
Step '收進 release\'

$targetDir = 'src-tauri\target\release'
$outDir = 'release'
if (-not (Test-Path $outDir)) { New-Item -ItemType Directory -Path $outDir | Out-Null }

$setupSource = Get-ChildItem "$targetDir\bundle\nsis\*_x64-setup.exe" | Select-Object -First 1
if (-not $setupSource) { Fail '找不到 NSIS 安裝檔' }

$artifacts = @(
    @{ From = "$targetDir\rich3-patch.exe"; To = "$ProjectName-v$version-Portable.exe" }
    @{ From = $setupSource.FullName;        To = "$ProjectName-v$version-Setup.exe" }
)

foreach ($item in $artifacts) {
    if (-not (Test-Path $item.From)) { Fail "找不到產物：$($item.From)" }
    # 一律走 -LiteralPath，不讓 PowerShell 對檔名做萬用字元展開
    Copy-Item -LiteralPath $item.From -Destination (Join-Path $outDir $item.To) -Force
    Write-Host "      $($item.To)"
}

# ---------------------------------------------------------------
# 4. 簽章（選用）
#
#    自簽憑證只是讓 EXE 帶上發行者名稱，不具信任價值，使用者仍會看到
#    SmartScreen 警告——README 的常見問題有說明。
# ---------------------------------------------------------------
if ($Sign) {
    Step '簽章'

    if (-not (Test-Path 'Overmind.pfx')) {
        # 先找存放區裡既有的憑證再用。舊版腳本只要找不到 pfx 就再簽發一張，
        # 跑幾次就在存放區裡累積幾張同名憑證。
        $cert = Get-ChildItem Cert:\CurrentUser\My |
            Where-Object { $_.Subject -eq 'CN=Overmind' -and $_.HasPrivateKey -and $_.NotAfter -gt (Get-Date) } |
            Sort-Object NotAfter -Descending | Select-Object -First 1

        if ($cert) {
            Write-Host "      沿用既有憑證（有效期至 $($cert.NotAfter.ToString('yyyy-MM-dd'))）"
        } else {
            Write-Host '      存放區沒有可用憑證，簽發一張新的自簽憑證'
            $cert = New-SelfSignedCertificate -Type CodeSigningCert -Subject 'CN=Overmind' `
                -KeyExportPolicy Exportable -KeySpec Signature -KeyLength 2048 `
                -KeyAlgorithm RSA -HashAlgorithm SHA256 -NotAfter (Get-Date).AddYears(10) `
                -CertStoreLocation 'Cert:\CurrentUser\My'
        }

        $pfxPwd = ConvertTo-SecureString -String 'overmind' -Force -AsPlainText
        Export-PfxCertificate -Cert $cert -FilePath '.\Overmind.pfx' -Password $pfxPwd | Out-Null
        Write-Host '      已匯出 Overmind.pfx（已在 .gitignore 內）'
    }

    $signtool = Get-ChildItem 'C:\Program Files (x86)\Windows Kits\10\bin\*\x64\signtool.exe' -ErrorAction SilentlyContinue |
        Sort-Object FullName -Descending | Select-Object -First 1

    if ($signtool) {
        foreach ($item in $artifacts) {
            $path = Join-Path $outDir $item.To
            $previous = $ErrorActionPreference
            $ErrorActionPreference = 'Continue'
            & $signtool.FullName sign /f 'Overmind.pfx' /p 'overmind' /fd SHA256 `
                /t http://timestamp.digicert.com /d $AppLabel $path
            $code = $LASTEXITCODE
            $ErrorActionPreference = $previous
            if ($code -ne 0) { Write-Host "[WARN] $($item.To) 簽章失敗（離開碼 $code）" -ForegroundColor Yellow }
        }
    } else {
        Write-Host '[WARN] 找不到 signtool.exe，跳過簽章。' -ForegroundColor Yellow
    }
}

# ---------------------------------------------------------------
# 5. 校驗碼 —— RELEASE_RULES.md §4.3 規定為必附項
#
#    必須在簽章之後計算，否則雜湊對不上實際上傳的檔案。
# ---------------------------------------------------------------
Step '產生 SHA256SUMS.txt'

$lines = foreach ($item in $artifacts) {
    $path = Join-Path $outDir $item.To
    $hash = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLower()
    "$hash  $($item.To)"
}
[System.IO.File]::WriteAllLines(
    (Join-Path (Get-Location) "$outDir\SHA256SUMS.txt"),
    $lines,
    (New-Object System.Text.UTF8Encoding($false))
)
$lines | ForEach-Object { Write-Host "      $_" }

# ---------------------------------------------------------------
# 6. 回報體積，隨時盯著不要肥起來
# ---------------------------------------------------------------
Write-Host ''
foreach ($item in $artifacts) {
    $size = (Get-Item -LiteralPath (Join-Path $outDir $item.To)).Length / 1MB
    Write-Host ('[SIZE] {0,-44} {1,6:N2} MB' -f $item.To, $size)
}
Write-Host ''
Write-Host "[DONE] 完工！產物在 $outDir\" -ForegroundColor Green
if (-not $Sign) { Write-Host '       （未簽章。要簽章請加 -Sign）' -ForegroundColor DarkGray }
