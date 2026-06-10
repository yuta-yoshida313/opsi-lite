# Opsi-Lite アプリアイコン生成スクリプト。
# 角丸グラデーション(インディゴ→シアン)の上に、白のMarkdown風マーク(M + 下向き矢印)。
# 256pxのPNGと、複数解像度を内包する .ico を出力する。
Add-Type -AssemblyName System.Drawing

function New-IconBitmap([int]$size) {
    $s = $size / 256.0
    $bmp = New-Object System.Drawing.Bitmap($size, $size, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $g.Clear([System.Drawing.Color]::Transparent)

    # 角丸矩形パス
    $pad = [float](16 * $s)
    $rad = [float](56 * $s)
    $x = $pad; $y = $pad; $w = $size - 2*$pad; $h = $size - 2*$pad
    $path = New-Object System.Drawing.Drawing2D.GraphicsPath
    $d = $rad * 2
    $path.AddArc($x, $y, $d, $d, 180, 90)
    $path.AddArc($x + $w - $d, $y, $d, $d, 270, 90)
    $path.AddArc($x + $w - $d, $y + $h - $d, $d, $d, 0, 90)
    $path.AddArc($x, $y + $h - $d, $d, $d, 90, 90)
    $path.CloseFigure()

    # 斜めグラデーション(indigo #4F46E5 -> cyan #06B6D4)
    $r1 = New-Object System.Drawing.RectangleF($x, $y, $w, $h)
    $c1 = [System.Drawing.Color]::FromArgb(255, 99, 102, 241)   # indigo-500ish
    $c2 = [System.Drawing.Color]::FromArgb(255, 6, 182, 212)    # cyan-500
    $brush = New-Object System.Drawing.Drawing2D.LinearGradientBrush($r1, $c1, $c2, 50.0)
    $g.FillPath($brush, $path)

    # 上部のハイライト(微光沢)
    $hl = [System.Drawing.Color]::FromArgb(46, 255, 255, 255)
    $hlBrush = New-Object System.Drawing.SolidBrush($hl)
    $hlPath = New-Object System.Drawing.Drawing2D.GraphicsPath
    $hlPath.AddArc($x, $y, $d, $d, 180, 90)
    $hlPath.AddArc($x + $w - $d, $y, $d, $d, 270, 90)
    $hlPath.AddLine($x + $w, $y + $h*0.42, $x, $y + $h*0.30)
    $hlPath.CloseFigure()
    $g.FillPath($hlBrush, $hlPath)

    # 白マーク: Markdown の "M" + 下向き矢印
    $white = [System.Drawing.Color]::FromArgb(255, 255, 255, 255)
    $pen = New-Object System.Drawing.Pen($white, [float](22 * $s))
    $pen.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
    $pen.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
    $pen.LineJoin = [System.Drawing.Drawing2D.LineJoin]::Round

    function P([float]$px, [float]$py) { New-Object System.Drawing.PointF([float]($px*$s), [float]($py*$s)) }

    # "M" (左)
    $mPts = @(
        (P 70 176), (P 70 92), (P 104 132), (P 138 92), (P 138 176)
    )
    $g.DrawLines($pen, [System.Drawing.PointF[]]$mPts)

    # 下向き矢印の縦棒 (右)
    $g.DrawLine($pen, (P 178 92), (P 178 150))
    # 矢印の頭(塗り三角)
    $tri = @( (P 160 140), (P 196 140), (P 178 178) )
    $triBrush = New-Object System.Drawing.SolidBrush($white)
    $g.FillPolygon($triBrush, [System.Drawing.PointF[]]$tri)

    $g.Dispose()
    return $bmp
}

$dir = $PSScriptRoot
if ([string]::IsNullOrEmpty($dir)) { $dir = "C:\Users\吉田裕太\Co\Opsi-Lite\assets" }
$png = Join-Path $dir "icon.png"
$ico = Join-Path $dir "icon.ico"

# 256px PNG
$big = New-IconBitmap 256
$big.Save($png, [System.Drawing.Imaging.ImageFormat]::Png)

# 256px raw RGBA (iced の window::icon::from_rgba 用)。GDIはBGRAなのでR/Bを入替。
$rgbaPath = $png -replace '\.png$', '.rgba'
$rect = New-Object System.Drawing.Rectangle(0, 0, 256, 256)
$bd = $big.LockBits($rect, [System.Drawing.Imaging.ImageLockMode]::ReadOnly, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
$len = $bd.Stride * $bd.Height
$buf = New-Object byte[] $len
[System.Runtime.InteropServices.Marshal]::Copy($bd.Scan0, $buf, 0, $len)
$big.UnlockBits($bd)
$out = New-Object byte[] (256*256*4)
for ($i = 0; $i -lt 256*256; $i++) {
    $o = $i * 4
    $out[$o]     = $buf[$o + 2]  # R
    $out[$o + 1] = $buf[$o + 1]  # G
    $out[$o + 2] = $buf[$o]      # B
    $out[$o + 3] = $buf[$o + 3]  # A
}
[System.IO.File]::WriteAllBytes($rgbaPath, $out)

# 複数サイズPNGを .ico に詰める(Vista+ はICO内PNGに対応)
$sizes = @(16,24,32,48,64,128,256)
$pngBytes = @()
foreach ($sz in $sizes) {
    $b = New-IconBitmap $sz
    $ms = New-Object System.IO.MemoryStream
    $b.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
    $pngBytes += ,($ms.ToArray())
    $ms.Dispose(); $b.Dispose()
}

$fs = New-Object System.IO.FileStream($ico, [System.IO.FileMode]::Create)
$bw = New-Object System.IO.BinaryWriter($fs)
# ICONDIR
$bw.Write([UInt16]0)      # reserved
$bw.Write([UInt16]1)      # type = icon
$bw.Write([UInt16]$sizes.Count)
$offset = 6 + 16 * $sizes.Count
for ($i = 0; $i -lt $sizes.Count; $i++) {
    $sz = $sizes[$i]
    $data = $pngBytes[$i]
    $wb = if ($sz -ge 256) { 0 } else { $sz }
    $bw.Write([Byte]$wb)     # width
    $bw.Write([Byte]$wb)     # height
    $bw.Write([Byte]0)       # colors
    $bw.Write([Byte]0)       # reserved
    $bw.Write([UInt16]1)     # planes
    $bw.Write([UInt16]32)    # bpp
    $bw.Write([UInt32]$data.Length)
    $bw.Write([UInt32]$offset)
    $offset += $data.Length
}
foreach ($data in $pngBytes) { $bw.Write($data) }
$bw.Flush(); $bw.Close(); $fs.Close()
$big.Dispose()
Write-Output "Wrote $png and $ico"
