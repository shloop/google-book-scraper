$toolsDir = (Split-Path -parent $MyInvocation.MyCommand.Definition)
$z32="$toolsDir/google-book-scraper-$VERSION-windows-x86-portable.zip"
$z64="$toolsDir/google-book-scraper-$VERSION-windows-x64-portable.zip"
Get-ChocolateyUnzip -FileFullPath $z32 -FileFullPath64 $z64 -Destination $toolsDir
Remove-Item $z32
Remove-Item $z64
Remove-Item $toolsDir/LICENSE-APACHE
Remove-Item $toolsDir/LICENSE-MIT