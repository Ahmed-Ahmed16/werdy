$ErrorActionPreference = 'Stop'
$repo = Split-Path $PSScriptRoot -Parent
$fixture = Join-Path $repo ('target/signing-tests-' + [guid]::NewGuid())
$variables = @('WERDY_ANDROID_KEYSTORE', 'WERDY_ANDROID_STORE_PASSWORD', 'WERDY_ANDROID_KEY_PASSWORD', 'CARGO_TARGET_DIR', 'GRADLE_USER_HOME')
$savedEnvironment = @{}
foreach ($name in $variables) { $savedEnvironment[$name] = [Environment]::GetEnvironmentVariable($name) }

function Assert-True($Condition, $Message) {
    if (-not $Condition) { throw $Message }
}

# dx is the external build boundary. These tests never invoke the installed CLI.
function global:dx {
    $global:signingTestState.Arguments = @($args)
    $global:signingTestState.Config = Get-Content crates/werdy-app/Dioxus.toml -Raw
    Assert-True (Test-Path '.android-signing/release.jks') 'Staged key missing'
    Assert-True ($env:CARGO_TARGET_DIR -eq (Join-Path $fixture 'target-release-signing')) 'Build escaped fixture'
    $generated = Join-Path $env:CARGO_TARGET_DIR 'dx/werdy-app/release/android/app/app'
    $keyPath = [regex]::Match($global:signingTestState.Config, '(?m)^jks_file = "([^"]+)"').Groups[1].Value
    $gradleKeyPath = [IO.Path]::GetFullPath((Join-Path $generated "../../../../../../../$keyPath"))
    Assert-True (Test-Path -LiteralPath $gradleKeyPath) 'Gradle template cannot reach staged key'
    Assert-True ($global:signingTestState.Config -match '(?m)^key_alias = "werdy"') 'Release alias missing'
    New-Item -ItemType Directory -Path $generated -Force | Out-Null
    Set-Content (Join-Path $generated 'build.gradle.kts') 'simulated signing secret'
    New-Item -ItemType Directory -Path $env:GRADLE_USER_HOME -Force | Out-Null
    Set-Content (Join-Path $env:GRADLE_USER_HOME 'cache') 'simulated compiled secret'
    if ($global:signingTestState.Fail) { $global:LASTEXITCODE = 7; return }
    # Write a stand in APK whose native code folder names the built CPU.
    $outDir = $args[[array]::IndexOf($args, '--out-dir') + 1]
    New-Item -ItemType Directory -Path $outDir -Force | Out-Null
    $abi = if ($global:signingTestState.Abi) { $global:signingTestState.Abi } else { 'arm64-v8a' }
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $apk = Join-Path $outDir 'app-release.apk'
    Remove-Item -LiteralPath $apk -ErrorAction SilentlyContinue
    $zip = [IO.Compression.ZipFile]::Open($apk, 'Create')
    try { $null = $zip.CreateEntry("lib/$abi/libmain.so") } finally { $zip.Dispose() }
    $global:LASTEXITCODE = 0
}

try {
    New-Item -ItemType Directory -Path "$fixture/scripts", "$fixture/crates/werdy-app" -Force | Out-Null
    Copy-Item "$PSScriptRoot/build-android-release.ps1" "$fixture/scripts/"
    Copy-Item "$repo/crates/werdy-app/Dioxus.toml" "$fixture/crates/werdy-app/"
    $configPath = Join-Path $fixture 'crates/werdy-app/Dioxus.toml'
    $original = [IO.File]::ReadAllBytes($configPath)
    $env:WERDY_ANDROID_KEYSTORE = Join-Path $fixture 'owner key.jks'
    Set-Content -LiteralPath $env:WERDY_ANDROID_KEYSTORE 'test fixture, not a keystore'
    $env:WERDY_ANDROID_STORE_PASSWORD = 'test $dollar "quotes" & <tags> ''apostrophe'' \ slash العربية'
    $env:WERDY_ANDROID_KEY_PASSWORD = 'different test password'
    $env:CARGO_TARGET_DIR = 'previous target'
    $env:GRADLE_USER_HOME = 'previous gradle home'

    foreach ($failBuild in @($false, $true)) {
        $global:signingTestState = @{ Fail = $failBuild }
        $failed = $false
        try { & "$fixture/scripts/build-android-release.ps1" } catch {
            $failed = $true
            if ($failBuild) { Assert-True ($_.Exception.Message -like '*exit code 7*') 'Unexpected build failure' }
        }
        Assert-True ($failed -eq $failBuild) 'Build failure was lost or success failed'
        Assert-True ([Convert]::ToBase64String([IO.File]::ReadAllBytes($configPath)) -eq [Convert]::ToBase64String($original)) 'Original config not restored'
        Assert-True (-not (Test-Path "$fixture/.android-signing")) 'Signing files survived'
        Assert-True (-not (Test-Path "$fixture/target-release-signing")) 'Generated secrets survived'
        Assert-True ($env:CARGO_TARGET_DIR -eq 'previous target') 'Target environment not restored'
        Assert-True ($env:GRADLE_USER_HOME -eq 'previous gradle home') 'Gradle environment not restored'
        $arguments = $global:signingTestState.Arguments -join ' '
        Assert-True ($arguments -like 'bundle --platform android --release*') 'Release bundle flags missing'
        Assert-True ($arguments -like '*--package-types apk --package-types aab*') 'APK or AAB missing'
        Assert-True ($arguments -like '*--target aarch64-linux-android*') 'Phone CPU target missing'
        $config = $global:signingTestState.Config
        foreach ($field in @{ jks_password = $env:WERDY_ANDROID_STORE_PASSWORD; key_password = $env:WERDY_ANDROID_KEY_PASSWORD }.GetEnumerator()) {
            $match = [regex]::Match($config, "(?m)^$($field.Key) = '([^']*)'")
            $decoded = [regex]::Replace($match.Groups[1].Value, '\\u([0-9a-f]{4})', { param($m) [string][char][Convert]::ToInt32($m.Groups[1].Value, 16) })
            Assert-True ($decoded -ceq $field.Value) 'Password changed during template encoding'
        }
        Write-Output "PASS: build failure=$failBuild preserves config and environment, removes secrets, requests release APK and AAB"
    }

    # The v0.1.0 APK held only x86_64 code, so phones refused it (#84).
    $global:signingTestState = @{ Abi = 'x86_64' }
    $failed = $false
    try { & "$fixture/scripts/build-android-release.ps1" } catch {
        $failed = $true
        Assert-True ($_.Exception.Message -like '*arm64-v8a*') 'Unexpected CPU check failure'
    }
    Assert-True $failed 'APK without phone code accepted'
    Assert-True (-not (Test-Path "$fixture/.android-signing")) 'Signing files survived CPU check'
    Assert-True ([Convert]::ToBase64String([IO.File]::ReadAllBytes($configPath)) -eq [Convert]::ToBase64String($original)) 'Config not restored after CPU check'
    Write-Output 'PASS: an APK without arm64-v8a code fails the build'

    foreach ($name in @('WERDY_ANDROID_KEYSTORE', 'WERDY_ANDROID_STORE_PASSWORD', 'WERDY_ANDROID_KEY_PASSWORD')) {
        $saved = [Environment]::GetEnvironmentVariable($name)
        [Environment]::SetEnvironmentVariable($name, '')
        $global:signingTestState = @{}
        $failed = $false
        try { & "$fixture/scripts/build-android-release.ps1" } catch { $failed = $true }
        Assert-True $failed "Missing $name accepted"
        Assert-True (-not $global:signingTestState.ContainsKey('Arguments')) 'Build ran without signing input'
        Assert-True (-not (Test-Path "$fixture/.android-signing")) 'Validation changed files'
        [Environment]::SetEnvironmentVariable($name, $saved)
        Write-Output "PASS: missing $name stops before build"
    }

    $env:WERDY_ANDROID_KEYSTORE = Join-Path $fixture 'missing.jks'
    $failed = $false
    try { & "$fixture/scripts/build-android-release.ps1" } catch { $failed = $true }
    Assert-True $failed 'Missing keystore accepted'
    Assert-True (-not (Test-Path "$fixture/.android-signing")) 'Missing keystore changed files'
    $env:WERDY_ANDROID_KEYSTORE = Join-Path $fixture 'owner key.jks'
    Write-Output 'PASS: nonexistent keystore stops before build'

    New-Item -ItemType Directory -Path "$fixture/.android-signing" | Out-Null
    Set-Content "$fixture/.android-signing/owner-file" 'must survive'
    $failed = $false
    try { & "$fixture/scripts/build-android-release.ps1" } catch { $failed = $true }
    Assert-True $failed 'Existing signing workspace accepted'
    Assert-True ((Get-Content "$fixture/.android-signing/owner-file") -eq 'must survive') 'Existing workspace changed'
    Write-Output 'PASS: existing signing workspace survives refused build'
} finally {
    foreach ($name in $variables) { [Environment]::SetEnvironmentVariable($name, $savedEnvironment[$name]) }
    Remove-Item Function:/dx -ErrorAction SilentlyContinue
    Remove-Variable signingTestState -Scope Global -ErrorAction SilentlyContinue
    if ([IO.Path]::GetFullPath($fixture).StartsWith((Join-Path $repo 'target/'), [StringComparison]::OrdinalIgnoreCase)) {
        if (Test-Path -LiteralPath $fixture) { Remove-Item -LiteralPath $fixture -Recurse -Force }
    }
}
