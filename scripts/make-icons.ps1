Add-Type -AssemblyName System.Drawing

$icons = Join-Path $PSScriptRoot '..\crates\werdy-app\public\icons'
New-Item -ItemType Directory -Force -Path $icons | Out-Null

foreach ($spec in @(
    @{ Name = 'icon-192.png'; Size = 192; Maskable = $false },
    @{ Name = 'icon-512.png'; Size = 512; Maskable = $false },
    @{ Name = 'icon-maskable-512.png'; Size = 512; Maskable = $true }
)) {
    $bitmap = [System.Drawing.Bitmap]::new($spec.Size, $spec.Size)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $graphics.ScaleTransform($spec.Size / 512, $spec.Size / 512)
    $white = [System.Drawing.Brushes]::White
    $green = [System.Drawing.SolidBrush]::new([System.Drawing.ColorTranslator]::FromHtml('#0F5C45'))
    try {
        if ($spec.Maskable) {
            $graphics.FillRectangle($green, 0, 0, 512, 512)
        } else {
            $path = [System.Drawing.Drawing2D.GraphicsPath]::new()
            $path.AddArc(0, 0, 192, 192, 180, 90)
            $path.AddArc(320, 0, 192, 192, 270, 90)
            $path.AddArc(320, 320, 192, 192, 0, 90)
            $path.AddArc(0, 320, 192, 192, 90, 90)
            $path.CloseFigure()
            $graphics.FillPath($green, $path)
            $path.Dispose()
        }
        foreach ($y in @(170, 256, 342)) {
            $graphics.FillEllipse($white, 132, $y - 24, 48, 48)
        }
        foreach ($row in @(
            @{ Y = 150; Width = 152 },
            @{ Y = 236; Width = 152 },
            @{ Y = 322; Width = 112 }
        )) {
            $graphics.FillRectangle($white, 224, $row.Y, $row.Width - 40, 40)
            $graphics.FillEllipse($white, 204, $row.Y, 40, 40)
            $graphics.FillEllipse($white, 204 + $row.Width - 40, $row.Y, 40, 40)
        }
        $bitmap.Save((Join-Path $icons $spec.Name), [System.Drawing.Imaging.ImageFormat]::Png)
    } finally {
        $green.Dispose()
        $graphics.Dispose()
        $bitmap.Dispose()
    }
}
