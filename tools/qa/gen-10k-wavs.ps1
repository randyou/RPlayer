$dir = 'D:\Workspace\RPlayer\testdata\perf10k'
if (Test-Path $dir) { Remove-Item $dir -Recurse -Force }
New-Item -ItemType Directory -Force $dir | Out-Null
# Minimal valid 16-bit PCM WAV: 44.1 kHz mono, 0.05 s of silence.
$sampleRate = 44100
$samples = [int]($sampleRate * 0.05)
$bytesPerSample = 2
$dataBytes = $samples * $bytesPerSample
$byteRate = $sampleRate * $bytesPerSample
$blockAlign = $bytesPerSample
$ms = New-Object System.IO.MemoryStream
$bw = New-Object System.IO.BinaryWriter($ms)
$bw.Write([Text.Encoding]::ASCII.GetBytes('RIFF'))
$bw.Write([int](36 + $dataBytes))
$bw.Write([Text.Encoding]::ASCII.GetBytes('WAVE'))
$bw.Write([Text.Encoding]::ASCII.GetBytes('fmt '))
$bw.Write([int]16)
$bw.Write([int16]1)
$bw.Write([int16]1)
$bw.Write([int]$sampleRate)
$bw.Write([int]$byteRate)
$bw.Write([int16]$blockAlign)
$bw.Write([int16]16)
$bw.Write([Text.Encoding]::ASCII.GetBytes('data'))
$bw.Write([int]$dataBytes)
$bw.Write([byte[]]::new($dataBytes))
$bw.Flush()
$bytes = $ms.ToArray()
$bw.Dispose(); $ms.Dispose()
[IO.File]::WriteAllBytes((Join-Path $dir 'probe.wav'), $bytes)
$sw = [Diagnostics.Stopwatch]::StartNew()
for ($i = 1; $i -le 10000; $i++) {
    [IO.File]::WriteAllBytes((Join-Path $dir ("perf-{0:d5}.wav" -f $i)), $bytes)
}
$sw.Stop()
"generated 10001 files x $($bytes.Length) B in $([int]$sw.Elapsed.TotalSeconds)s"
