[Setup]
AppName=Google Book Scraper
AppId=gbscraper
AppVersion="{#GetEnv('VERSION')}"
AppPublisher=shloop
AppPublisherURL=https://github.com/shloop/google-book-scraper
AppSupportURL=https://github.com/shloop/google-book-scraper/issues
DefaultDirName={autopf}\gbscraper
DefaultGroupName=gbscraper
ArchitecturesAllowed="{#GetEnv('ARCH')}"
; ArchitecturesAllowed=x64compatible and not arm64
ArchitecturesInstallIn64BitMode=arm64 x64compatible
Compression=zip

[Files]
Source: "gbscraper.exe"; DestDir: "{app}"
Source: "LICENSE-APACHE"; DestDir: "{app}"
Source: "LICENSE-MIT"; DestDir: "{app}"
Source: "attribution.txt"; DestDir: "{app}"
Source: "README.md"; DestDir: "{app}"; Flags: isreadme

[Code]

function NeedsAddPath(Param: string): boolean;
var
  OrigPath: string;
begin
  if not RegQueryStringValue(HKEY_LOCAL_MACHINE,
    'SYSTEM\CurrentControlSet\Control\Session Manager\Environment',
    'Path', OrigPath)
  then begin
    Result := True;
    exit;
  end;
  { look for the path with leading and trailing semicolon }
  { Pos() returns 0 if not found }
  Result := Pos(';' + Param + ';', ';' + OrigPath + ';') = 0;
end;

[Registry]
Root: "HKLM"; Subkey: "SYSTEM\CurrentControlSet\Control\Session Manager\Environment"; ValueType: expandsz; ValueName: "Path"; ValueData: "{olddata};{app}"; Check: NeedsAddPath(ExpandConstant('{app}'))