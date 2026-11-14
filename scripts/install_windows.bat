@echo off
setlocal enabledelayedexpansion
echo ========================================
echo   Ferrox Browser - Windows Installer
echo ========================================
echo.

:: Check if running as administrator
net session >nul 2>&1
if %errorLevel% neq 0 (
    echo [!] This installer requires administrator privileges.
    echo     Please right-click and run as Administrator.
    echo.
    pause
    exit /b 1
)

:: Determine source location
set "SOURCE=%~dp0ferrox.exe"
set "DEST=C:\Program Files\Ferrox"
set "DESKTOP=%USERPROFILE%\Desktop"

if not exist "%SOURCE%" (
    echo [!] ferrox.exe not found in this directory!
    echo     Please extract ferrox-windows-x86_64.zip first.
    echo.
    pause
    exit /b 1
)

echo [1/4] Creating installation directory...
if not exist "%DEST%" (
    mkdir "%DEST%"
    echo      Created: %DEST%
) else (
    echo      Exists: %DEST%
)

echo [2/4] Copying ferrox.exe...
copy /Y "%SOURCE%" "%DEST%\ferrox.exe" >nul
if errorlevel 1 (
    echo [ERROR] Failed to copy file!
    pause
    exit /b 1
)
echo      Copied: %DEST%\ferrox.exe

echo [3/4] Verifying integrity...
certutil -hashfile "%DEST%\ferrox.exe" SHA256 > "%DEST%\ferrox.exe.sha256"
echo      Checksum saved to: %DEST%\ferrox.exe.sha256

echo [4/4] Creating desktop shortcut...
powershell -Command "^$WshShell = New-Object -ComObject WScript.Shell; ^$Shortcut = ^$WshShell.CreateShortcut('%DESKTOP%\Ferrox Browser.lnk'); ^$Shortcut.TargetPath = '%DEST%\ferrox.exe'; ^$Shortcut.WorkingDirectory = '%DEST%'; ^$Shortcut.Description = 'Ferrox Browser'; ^$Shortcut.Save()" >nul 2>&1
if errorlevel 1 (
    echo      [!] Could not create desktop shortcut (Windows PowerShell issue)
) else (
    echo      Desktop shortcut created
)

echo.
echo ========================================
echo   Ferrox Browser is installed!
echo   Launch it from Desktop or %DEST%
echo ========================================
echo.
pause
exit /b 0
