# -*- mode: python ; coding: utf-8 -*-
# Rich Patch Series — PyInstaller 打包設定

# 一律排除的肥胖模組，避免 EXE 無謂膨脹到好幾 MB
EXCLUDES = [
    'numpy', 'scipy', 'pandas', 'matplotlib', 'PIL', 'cv2',
    'PyQt5', 'PyQt6', 'PySide2', 'PySide6', 'wx',
    'IPython', 'jupyter', 'notebook', 'pytest', 'setuptools', 'pip',
    'test', 'unittest', 'pydoc', 'doctest', 'lib2to3', 'distutils',
    'sqlite3', 'xml', 'xmlrpc', 'email', 'html', 'http', 'urllib', 'urllib3',
    'multiprocessing', 'asyncio', 'curses', 'bz2', 'lzma', 'ssl', 'socket',
]

# Tcl/Tk 內建一堆本程式用不到的資源，整包砍掉可省下約 2.5 MB
TRIM_DIRS = (
    '_tcl_data/tzdata',   # 時區資料庫，我們沒用 Tcl 的 clock
    '_tcl_data/msgs',     # Tcl 語系訊息
    '_tcl_data/opt0.4',
    '_tcl_data/http1.0',
    '_tk_data/images',    # Tk 內建示範圖片
    '_tk_data/msgs',      # Tk 語系訊息
)

# encoding 整包 1.5 MB，只留 Tcl 啟動與繁中環境真正會用到的
KEEP_ENCODINGS = {
    'ascii.enc', 'utf-8.enc', 'unicode.enc', 'iso8859-1.enc',
    'cp1252.enc', 'cp950.enc', 'cp936.enc', 'cp437.enc', 'big5.enc',
}

def trim_datas(datas):
    kept = []
    for entry in datas:
        dest = entry[0].replace('\\', '/')
        if dest.startswith(TRIM_DIRS):
            continue
        if '/encoding/' in dest and dest.rsplit('/', 1)[-1] not in KEEP_ENCODINGS:
            continue
        kept.append(entry)
    return kept

a = Analysis(
    ['main.py'],
    pathex=[],
    binaries=[],
    datas=[('icon.png', '.'), ('EVENTVOC', 'EVENTVOC'), ('NEWSVOC', 'NEWSVOC'), ('SCREEN', 'SCREEN')],
    hiddenimports=[],
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=EXCLUDES,
    noarchive=False,
    optimize=2,
)
a.datas = trim_datas(a.datas)

pyz = PYZ(a.pure)

exe = EXE(
    pyz,
    a.scripts,
    a.binaries,
    a.datas,
    [],
    name='rich3_patch',
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=True,
    upx_exclude=[],
    runtime_tmpdir=None,
    console=False,
    disable_windowed_traceback=False,
    argv_emulation=False,
    target_arch=None,
    codesign_identity=None,
    entitlements_file=None,
    version='file_version_info.txt',
    icon=['icon.ico'],
)
