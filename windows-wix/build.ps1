$ErrorActionPreference = "Stop"

if (Test-Path $PSScriptRoot\build) {
	Remove-Item -Recurse -Force $PSScriptRoot\build
}

mkdir $PSScriptRoot\build

Set-Location "$PSScriptRoot"

dotnet tool restore
dotnet tool run wix -acceptEula wix7 extension add WixToolset.UI.wixext/7.0.0
dotnet tool run wix -acceptEula wix7 extension add WixToolset.Util.wixext/7.0.0

dotnet tool run wix build -acceptEula wix7 `
	-arch x64 -ext WixToolset.UI.wixext -ext WixToolset.Util.wixext `
	-d Version="$Env:FEND_VERSION" `
	-d FendExePath="$PSScriptRoot\..\target\release\fend.exe" `
	-d LicenseMdPath="$PSScriptRoot\..\LICENSE.md" `
	-d IconPath="$PSScriptRoot\..\icon\fend-icon.ico" `
	-o "$PSScriptRoot\build\fend-windows-x64.msi" "$PSScriptRoot\main.wxs"
