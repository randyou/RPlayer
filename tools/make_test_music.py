"""Generate small tagged WAV files for local UI / playback testing."""

from __future__ import annotations

import math
import struct
import wave
from pathlib import Path

SR = 44_100
OUT = Path(__file__).resolve().parent.parent / "testdata"

TRACKS = [
    ("midnight-neon.wav", "午夜霓虹", "林间电台", "城市夜航", 220.0, 24),
    ("harbor-letters.wav", "海风与旧信", "苏晚", "潮汐来信", 261.63, 28),
    ("after-rain.wav", "雨后站台", "陈屿", "慢行列车", 196.0, 22),
    ("starry-dream.wav", "星河入梦", "白昼漫游", "凌晨两点", 293.66, 31),
    ("summer-echo.wav", "夏日回声", "橘子汽水", "晴日手记", 329.63, 19),
]


def info_chunk(title: str, artist: str, album: str) -> bytes:
    def sub(tag: bytes, text: str) -> bytes:
        data = text.encode("utf-8") + b"\x00"
        pad = b"\x00" if len(data) % 2 else b""
        return tag + struct.pack("<I", len(data)) + data + pad

    body = b"INFO" + sub(b"INAM", title) + sub(b"IART", artist) + sub(b"IPRD", album)
    return b"LIST" + struct.pack("<I", len(body)) + body


def render(freq: float, seconds: int) -> bytes:
    total = SR * seconds
    frames = bytearray()
    chords = [1.0, 1.25, 1.5, 1.333]
    for i in range(total):
        t = i / SR
        # Slow four-bar harmonic movement under a simple bell-like melody.
        chord = chords[int(t / 4) % len(chords)]
        swell = min(1.0, t / 2.0) * min(1.0, (seconds - t) / 2.5)
        beat = 0.5 + 0.5 * math.sin(2 * math.pi * 0.5 * t)
        tone = (
            0.34 * math.sin(2 * math.pi * freq * chord * t)
            + 0.18 * math.sin(2 * math.pi * freq * chord * 2.0 * t)
            + 0.10 * math.sin(2 * math.pi * freq * chord * 3.0 * t)
        )
        melody_phase = int(t / 0.75) % 8
        melody_freq = freq * 2.0 * [1.0, 1.125, 1.25, 1.5, 1.25, 1.125, 1.0, 0.75][
            melody_phase
        ]
        melody = 0.15 * math.sin(2 * math.pi * melody_freq * t) * math.exp(
            -3.0 * (t % 0.75)
        )
        sample = (tone * beat + melody) * swell * 0.7
        left = max(-1.0, min(1.0, sample * 1.02))
        right = max(-1.0, min(1.0, sample * 0.96))
        frames += struct.pack("<hh", int(left * 32767), int(right * 32767))
    return bytes(frames)


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    for filename, title, artist, album, freq, seconds in TRACKS:
        pcm = render(freq, seconds)
        fmt = struct.pack("<4sIHHIIHH", b"fmt ", 16, 1, 2, SR, SR * 4, 4, 16)
        info = info_chunk(title, artist, album)
        data = struct.pack("<4sI", b"data", len(pcm)) + pcm
        riff_body = b"WAVE" + fmt + data + info
        path = OUT / filename
        with wave.open(str(path), "wb") as wav:
            wav.setnchannels(2)
            wav.setsampwidth(2)
            wav.setframerate(SR)
            wav.writeframes(pcm)
        # wave module cannot append LIST; rewrite the RIFF body with INFO after data.
        path.write_bytes(b"RIFF" + struct.pack("<I", len(riff_body)) + riff_body)
        print(f"{path.name}: {title} / {artist} / {album} ({seconds}s)")


if __name__ == "__main__":
    main()
