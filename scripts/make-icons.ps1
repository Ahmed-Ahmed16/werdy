$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$app = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../crates/werdy-app'))
$res = Join-Path $app 'src/android/src/main/res'
$source = Join-Path $app 'assets/icon/icon.svg'
[xml]$svg = Get-Content -LiteralPath $source -Raw
$tokens = Get-Content -LiteralPath (Join-Path $app 'assets/tokens.css') -Raw
$background = $svg.svg.rect.fill
$foreground = $svg.svg.g.fill
$stroke = $svg.svg.polyline.'stroke-width'
foreach ($pair in @(@('primary', $background), @('on-primary', $foreground))) {
    if ($tokens -notmatch "--$($pair[0]):\s*$($pair[1]);") { throw "Icon colour differs from token $($pair[0])" }
}

function Write-Asset([string]$Path, [string]$Text) {
    New-Item -ItemType Directory -Force -Path (Split-Path $Path) | Out-Null
    [System.IO.File]::WriteAllText($Path, $Text, [System.Text.UTF8Encoding]::new($false))
}

# This renderer supports the circles and polyline used by our source SVG.
function Write-Icon([string]$Path, [int]$Size) {
    New-Item -ItemType Directory -Force -Path (Split-Path $Path) | Out-Null
    $large = [System.Drawing.Bitmap]::new($Size * 4, $Size * 4)
    $graphics = [System.Drawing.Graphics]::FromImage($large)
    $brush = [System.Drawing.SolidBrush]::new([System.Drawing.ColorTranslator]::FromHtml($foreground))
    $pen = [System.Drawing.Pen]::new($brush, [single]$svg.svg.polyline.'stroke-width')
    $pen.StartCap = $pen.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
    $pen.LineJoin = [System.Drawing.Drawing2D.LineJoin]::Round
    try {
        $graphics.Clear([System.Drawing.ColorTranslator]::FromHtml($background))
        $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
        $graphics.ScaleTransform($Size * 4 / 108, $Size * 4 / 108)
        foreach ($circle in $svg.svg.g.circle) {
            $r = [single]$circle.r
            $graphics.FillEllipse($brush, ([single]$circle.cx - $r), ([single]$circle.cy - $r), (2 * $r), (2 * $r))
        }
        [System.Drawing.PointF[]]$points = @($svg.svg.polyline.points.Split(' ') | ForEach-Object {
            $xy = $_.Split(',')
            [System.Drawing.PointF]::new([single]$xy[0], [single]$xy[1])
        })
        $graphics.DrawLines($pen, $points)
        $small = [System.Drawing.Bitmap]::new($Size, $Size)
        $output = [System.Drawing.Graphics]::FromImage($small)
        try {
            $output.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
            $output.DrawImage($large, 0, 0, $Size, $Size)
            $small.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png)
        } finally { $output.Dispose(); $small.Dispose() }
    } finally { $pen.Dispose(); $brush.Dispose(); $graphics.Dispose(); $large.Dispose() }
}

foreach ($spec in @(@('icon-192.png', 192), @('icon-512.png', 512), @('icon-maskable-512.png', 512))) {
    Write-Icon (Join-Path $app "public/icons/$($spec[0])") $spec[1]
}
foreach ($spec in @(@('mdpi', 48), @('hdpi', 72), @('xhdpi', 96), @('xxhdpi', 144), @('xxxhdpi', 192))) {
    Write-Icon (Join-Path $res "mipmap-$($spec[0])/werdy_launcher.png") $spec[1]
}
Copy-Item -LiteralPath $source -Destination (Join-Path $app 'public/icons/favicon.svg')

$paths = foreach ($circle in $svg.svg.g.circle) {
    $x = [int]$circle.cx; $y = [int]$circle.cy; $r = [int]$circle.r
    '    <path android:fillColor="{0}" android:pathData="M{1},{2} a{3},{3} 0,1 0 {4},0 a{3},{3} 0,1 0 -{4},0"/>' -f $foreground, ($x-$r), $y, $r, ($r*2)
}
$check = 'M' + ($svg.svg.polyline.points -replace ' ', ' L')
Write-Asset (Join-Path $res 'drawable/werdy_foreground.xml') @"
<vector xmlns:android="http://schemas.android.com/apk/res/android" android:width="108dp" android:height="108dp" android:viewportWidth="108" android:viewportHeight="108">
$($paths -join "`n")
    <path android:fillColor="@android:color/transparent" android:strokeColor="$foreground" android:strokeWidth="$stroke" android:strokeLineCap="round" android:strokeLineJoin="round" android:pathData="$check"/>
</vector>
"@
Write-Asset (Join-Path $res 'values/werdy_icon.xml') ('<resources><color name="werdy_icon_background">{0}</color></resources>' -f $background)
Write-Asset (Join-Path $res 'mipmap-anydpi-v26/werdy_launcher.xml') @"
<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">
    <background android:drawable="@color/werdy_icon_background"/>
    <foreground android:drawable="@drawable/werdy_foreground"/>
</adaptive-icon>
"@

$config = Get-Content -LiteralPath (Join-Path $app 'Dioxus.toml') -Raw
if ($config -notmatch '(?m)^name\s*=\s*"([^"]+)"') { throw 'Missing application name' }
$name = [System.Security.SecurityElement]::Escape($Matches[1])
Write-Asset (Join-Path $app 'assets/icon/AndroidManifest.xml') @"
<manifest xmlns:android="http://schemas.android.com/apk/res/android">
    <uses-permission android:name="android.permission.INTERNET"/>
    <application android:hasCode="true" android:icon="@mipmap/werdy_launcher"
        android:roundIcon="@mipmap/werdy_launcher" android:label="$name"
        android:extractNativeLibs="true" android:allowNativeHeapPointerTagging="false"
        android:supportsRtl="true" android:theme="@style/AppTheme"
        android:networkSecurityConfig="@xml/network_security_config">
        <activity android:name="dev.dioxus.main.MainActivity" android:exported="true"
            android:configChanges="orientation|screenLayout|screenSize|keyboardHidden">
            <meta-data android:name="android.app.lib_name" android:value="main"/>
            <meta-data android:name="android.app.func_name" android:value="ANativeActivity_onCreate"/>
            <intent-filter>
                <action android:name="android.intent.action.MAIN"/>
                <category android:name="android.intent.category.LAUNCHER"/>
            </intent-filter>
        </activity>
    </application>
</manifest>
"@
