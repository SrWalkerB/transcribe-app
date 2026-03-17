#!/usr/bin/env python3
from __future__ import annotations

import os
import platform
import shutil
import stat
import sys
import tarfile
import tempfile
import urllib.request
import zipfile
from pathlib import Path


WINDOWS_URL = "https://github.com/BtbN/FFmpeg-Builds/releases/latest/download/ffmpeg-master-latest-win64-gpl.zip"
LINUX_X86_64_URL = "https://github.com/BtbN/FFmpeg-Builds/releases/latest/download/ffmpeg-master-latest-linux64-gpl.tar.xz"
LINUX_ARM64_URL = "https://github.com/BtbN/FFmpeg-Builds/releases/latest/download/ffmpeg-master-latest-linuxarm64-gpl.tar.xz"


def pick_download() -> tuple[str, str]:
    system = sys.platform.lower()
    arch = platform.machine().lower()

    if system.startswith("win"):
        return WINDOWS_URL, ".zip"

    if system.startswith("linux"):
        if arch in {"x86_64", "amd64"}:
            return LINUX_X86_64_URL, ".tar.xz"
        if arch in {"aarch64", "arm64"}:
            return LINUX_ARM64_URL, ".tar.xz"

        raise SystemExit(f"Arquitetura Linux não suportada: {platform.machine()}")

    raise SystemExit(f"Plataforma não suportada: {sys.platform}")


def download_file(url: str, destination: Path) -> None:
    with urllib.request.urlopen(url) as response, destination.open("wb") as file:
        shutil.copyfileobj(response, file)


def extract_archive(archive: Path, destination: Path) -> None:
    destination.mkdir(parents=True, exist_ok=True)
    suffix = "".join(archive.suffixes)

    if suffix.endswith(".zip"):
        with zipfile.ZipFile(archive) as zip_file:
            zip_file.extractall(destination)
        return

    if suffix.endswith(".tar.xz"):
        with tarfile.open(archive, mode="r:xz") as tar_file:
            tar_file.extractall(destination)
        return

    raise SystemExit(f"Formato de arquivo não suportado: {archive.name}")


def find_binary(root: Path, binary_name: str) -> Path:
    matches = []
    target = binary_name.lower()
    for path in root.rglob("*"):
        if path.is_file() and path.name.lower() == target:
            matches.append(path)

    if not matches:
        raise SystemExit(f"Não encontrei {binary_name} dentro do pacote baixado.")

    return matches[0]


def copy_binary(source: Path, target: Path) -> None:
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, target)
    if os.name != "nt":
        target.chmod(target.stat().st_mode | stat.S_IEXEC)


def main() -> int:
    if len(sys.argv) < 2:
        print("Uso: install_ffmpeg.py <diretório-de-instalação>", file=sys.stderr)
        return 1

    install_dir = Path(sys.argv[1]).expanduser().resolve()
    ffmpeg_name = "ffmpeg.exe" if os.name == "nt" else "ffmpeg"
    ffprobe_name = "ffprobe.exe" if os.name == "nt" else "ffprobe"

    url, suffix = pick_download()

    with tempfile.TemporaryDirectory() as tmp_dir:
        tmp_path = Path(tmp_dir)
        archive_path = tmp_path / f"ffmpeg{suffix}"
        extract_dir = tmp_path / "extract"

        print(f"Baixando FFmpeg de {url}...")
        download_file(url, archive_path)
        print("Extraindo pacote...")
        extract_archive(archive_path, extract_dir)

        ffmpeg_src = find_binary(extract_dir, ffmpeg_name)
        ffprobe_src = find_binary(extract_dir, ffprobe_name)

        copy_binary(ffmpeg_src, install_dir / ffmpeg_name)
        copy_binary(ffprobe_src, install_dir / ffprobe_name)

    print(f"FFmpeg instalado em {install_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
