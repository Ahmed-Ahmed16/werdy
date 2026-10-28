param(
    [ValidateSet('apk', 'aab')]
    [string[]]$PackageType = @('apk', 'aab')
)

$ErrorActionPreference = 'Stop'
$repo = Split-Path $PSScriptRoot -Parent
$configPath = Join-Path $repo 'crates/werdy-app/Dioxus.toml'
$signingDir = Join-Path $repo '.android-signing'
$buildDir = Join-Path $repo 'target-release-signing'
$backupPath = Join-Path $signingDir 'Dioxus.toml.backup'

foreach ($name in @('WERDY_ANDROID_KEYSTORE', 'WERDY_ANDROID_STORE_PASSWORD', 'WERDY_ANDROID_KEY_PASSWORD')) {
    if ([string]::IsNullOrEmpty([Environment]::GetEnvironmentVariable($name))) {
        throw "Set $name before building. See crates/werdy-app/README.md."
    }
}
if (-not (Test-Path -LiteralPath $env:WERDY_ANDROID_KEYSTORE -PathType Leaf)) {
    throw 'WERDY_ANDROID_KEYSTORE must name an existing keystore file.'
}
$dxCommand = Get-Command dx -ErrorAction Stop
$baseConfig = [IO.File]::ReadAllText($configPath)
if ($baseConfig -match '(?m)^\s*\[bundle\.android\]') {
    throw 'Keep signing settings out of the tracked Dioxus.toml.'
}
if ((Test-Path -LiteralPath $signingDir) -or (Test-Path -LiteralPath $buildDir)) {
    throw 'Signing workspace exists. Follow interrupted build recovery in crates/werdy-app/README.md.'
}

function ConvertTo-GradleTomlString([string]$Password) {
    # Unicode escapes survive TOML literal strings and Handlebars HTML escaping.
    $escaped = ($Password.ToCharArray() | ForEach-Object { '\u{0:x4}' -f [int]$_ }) -join ''
    return "'$escaped'"
}

$storePassword = ConvertTo-GradleTomlString $env:WERDY_ANDROID_STORE_PASSWORD
$keyPassword = ConvertTo-GradleTomlString $env:WERDY_ANDROID_KEY_PASSWORD
$signingConfig = @"

[bundle.android]
jks_file = ".android-signing/release.jks"
jks_password = $storePassword
key_alias = "werdy"
key_password = $keyPassword
"@
$savedTarget = $env:CARGO_TARGET_DIR
$savedGradleHome = $env:GRADLE_USER_HOME
$locationPushed = $false
$buildDirCreated = $false
New-Item -ItemType Directory -Path $signingDir | Out-Null
try {
    Copy-Item -LiteralPath $configPath -Destination $backupPath
    Copy-Item -LiteralPath $env:WERDY_ANDROID_KEYSTORE -Destination (Join-Path $signingDir 'release.jks')
    New-Item -ItemType Directory -Path $buildDir | Out-Null
    $buildDirCreated = $true
    $env:CARGO_TARGET_DIR = $buildDir
    $env:GRADLE_USER_HOME = Join-Path $signingDir 'gradle-home'
    New-Item -ItemType Directory -Path $env:GRADLE_USER_HOME | Out-Null
    # A persistent daemon can keep secret caches locked on Windows.
    Set-Content -LiteralPath (Join-Path $env:GRADLE_USER_HOME 'gradle.properties') 'org.gradle.daemon=false'
    [IO.File]::WriteAllText($configPath, $baseConfig + "`n" + $signingConfig, [Text.UTF8Encoding]::new($false))
    Push-Location $repo
    $locationPushed = $true
    $buildArguments = @('bundle', '--platform', 'android', '--release', '--package', 'werdy-app', '--no-default-features', '--features', 'mobile')
    foreach ($format in $PackageType) { $buildArguments += @('--package-types', $format) }
    $buildArguments += @('--out-dir', (Join-Path $repo 'target/android-release'))
    & $dxCommand @buildArguments
    if ($LASTEXITCODE -ne 0) { throw "Android release build failed with exit code $LASTEXITCODE." }
} finally {
    if ($locationPushed) { Pop-Location }
    $env:CARGO_TARGET_DIR = $savedTarget
    $env:GRADLE_USER_HOME = $savedGradleHome
    if (Test-Path -LiteralPath $backupPath) {
        Copy-Item -LiteralPath $backupPath -Destination $configPath -Force
    }
    # Only directories created by this invocation belong to this cleanup.
    if ($buildDirCreated -and [IO.Path]::GetFullPath($buildDir).StartsWith($repo + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
        Remove-Item -LiteralPath $buildDir -Recurse -Force
    }
    if ([IO.Path]::GetFullPath($signingDir).StartsWith($repo + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
        Remove-Item -LiteralPath $signingDir -Recurse -Force
    }
}
