; The Windows download: one setup program that puts stackup.exe in the user's programs folder, adds
; that folder to the user's PATH so `stackup` works in any terminal, and makes a Start menu entry
; and an uninstaller. Built by the release workflow from dist\stackup once it holds stackup.exe:
;   ISCC.exe /DAppVersion=0.1.0 packaging\windows\stackup.iss

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif

[Setup]
; Never change it: Windows knows an installed stackup by it, to upgrade or remove it.
AppId={{6F2B8C1E-4D7A-4E59-9B3C-2A8E5F1D7C40}
AppName=stackup
AppVersion={#AppVersion}
AppVerName=stackup {#AppVersion}
AppPublisher=Marios Elef
AppPublisherURL=https://github.com/MariosElef/stackup
; For this user only, so it installs without asking for an administrator.
PrivilegesRequired=lowest
DefaultDirName={autopf}\stackup
DisableProgramGroupPage=yes
DisableDirPage=auto
UninstallDisplayIcon={app}\stackup.exe
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=..\..\dist
OutputBaseFilename=stackup-{#AppVersion}-windows-x64-setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
LicenseFile=..\..\LICENSE
; Open terminals hear that PATH changed.
ChangesEnvironment=yes
; stackup must be closed to be replaced.
CloseApplications=yes

[Files]
Source: "..\..\dist\stackup\stackup.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
; stackup is a terminal program: the entry opens it in a console window.
Name: "{autoprograms}\stackup"; Filename: "{app}\stackup.exe"; WorkingDir: "{%USERPROFILE}"

[Registry]
; The install folder on the user's PATH, once.
Root: HKCU; Subkey: "Environment"; ValueType: expandsz; ValueName: "Path"; \
  ValueData: "{olddata};{app}"; Check: NeedsAddPath(ExpandConstant('{app}'))

[Run]
Filename: "{app}\stackup.exe"; Description: "{cm:LaunchProgram,stackup}"; WorkingDir: "{%USERPROFILE}"; Flags: nowait postinstall skipifsilent

[Code]
function NeedsAddPath(Dir: string): Boolean;
var
  Path: string;
begin
  if not RegQueryStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', Path) then
  begin
    Result := True;
    exit;
  end;
  Result := Pos(';' + Uppercase(Dir) + ';', ';' + Uppercase(Path) + ';') = 0;
end;

{ Uninstalling takes the folder off the PATH again. }
procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  Path, Dir: string;
  P: Integer;
begin
  if CurUninstallStep <> usPostUninstall then
    exit;
  if not RegQueryStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', Path) then
    exit;
  Dir := ExpandConstant('{app}');
  P := Pos(';' + Uppercase(Dir), Uppercase(Path));
  if P > 0 then
  begin
    Delete(Path, P, Length(Dir) + 1);
    RegWriteExpandStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', Path);
  end;
end;
