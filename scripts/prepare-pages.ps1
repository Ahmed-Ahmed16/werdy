param(
    [Parameter(Mandatory = $true)]
    [string]$PublicDir
)

$ErrorActionPreference = 'Stop'
$root = (Resolve-Path -LiteralPath $PublicDir).Path
$required = @('index.html', 'manifest.webmanifest', 'icons/icon-192.png', 'icons/icon-512.png', 'icons/icon-maskable-512.png')
foreach ($name in $required) {
    if (-not (Test-Path -LiteralPath (Join-Path $root $name) -PathType Leaf)) {
        throw "Missing build file: $name"
    }
}

$files = @(Get-ChildItem -LiteralPath $root -Recurse -File | Where-Object {
    $_.Name -notin @('404.html', 'service-worker.js') -and
    $_.Extension -notin @('.map', '.br', '.gz')
} | Sort-Object FullName)
if (-not ($files | Where-Object Extension -EQ '.wasm') -or
    -not ($files | Where-Object Extension -EQ '.js') -or
    -not ($files | Where-Object Extension -EQ '.css')) {
    throw 'Build missing wasm, js, or css'
}

$urls = @($files | ForEach-Object {
    '/werdy/' + $_.FullName.Substring($root.Length + 1).Replace('\', '/')
})
$fingerprints = ($files | ForEach-Object { (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash }) -join ''
$hasher = [System.Security.Cryptography.SHA256]::Create()
try {
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($fingerprints)
    $version = [System.BitConverter]::ToString($hasher.ComputeHash($bytes)).Replace('-', '').Substring(0, 16).ToLowerInvariant()
} finally {
    $hasher.Dispose()
}

$template = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'service-worker.template.js') -Raw
$worker = $template.Replace('__VERSION__', $version).Replace('__PRECACHE__', (ConvertTo-Json -InputObject $urls -Compress))
[System.IO.File]::WriteAllText((Join-Path $root 'service-worker.js'), $worker, [System.Text.UTF8Encoding]::new($false))
Copy-Item -LiteralPath (Join-Path $root 'index.html') -Destination (Join-Path $root '404.html') -Force
