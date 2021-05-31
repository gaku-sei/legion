@echo off
SET LSC_BIN_DIR=%~dp0target\debug
rmdir /Q /S d:\temp\repo

SET WORKSPACE=d:\temp\workspace
rmdir /Q /S %WORKSPACE%

cargo build
IF %ERRORLEVEL% NEQ 0 exit /b 1

%LSC_BIN_DIR%\lsc-cli.exe init-local-repository -r d:\temp\repo
IF %ERRORLEVEL% NEQ 0 exit /b 1

%LSC_BIN_DIR%\lsc-cli.exe init-workspace -w d:\temp\workspace -r d:\temp\repo
IF %ERRORLEVEL% NEQ 0 exit /b 1

mkdir %WORKSPACE%\dir0

@rem adding using an absolute path from outside the workspace
copy %~dp0test.bat %WORKSPACE%\dir0\file0.txt
%LSC_BIN_DIR%\lsc-cli.exe add %WORKSPACE%\dir0\file0.txt
IF %ERRORLEVEL% NEQ 0 exit /b 1

@rem adding using a relative path from within the workspace
copy %~dp0test.bat %WORKSPACE%\dir0\file1.txt
pushd %WORKSPACE%\dir0
%LSC_BIN_DIR%\lsc-cli.exe add file1.txt
IF %ERRORLEVEL% NEQ 0 exit /b 1

%LSC_BIN_DIR%\lsc-cli.exe local-changes
IF %ERRORLEVEL% NEQ 0 exit /b 1
popd
