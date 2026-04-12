import sys
import os

if sys.platform == "win32":
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    sys.stderr.reconfigure(encoding="utf-8", errors="replace")
    os.environ.setdefault("HF_HUB_DISABLE_SYMLINKS_WARNING", "1")

import argparse
from faster_whisper import WhisperModel

parser = argparse.ArgumentParser()
parser.add_argument("mp3_name", help="Nome do arquivo MP3")
parser.add_argument("mp3_path", help="Caminho absoluto do MP3")
parser.add_argument("--model", default="base", choices=["tiny", "base", "small", "medium", "large", "turbo"])
parser.add_argument("--threads", type=int, default=4)
args = parser.parse_args()

file_name = "./output-text/{}.txt".format(args.mp3_name)

model = WhisperModel(args.model, device="cpu", compute_type="int8", cpu_threads=args.threads)

segments, info = model.transcribe(args.mp3_path, beam_size=5)

# Print duration so Rust can calculate progress
print("__DURATION__:%.2f" % info.duration, flush=True)
print("__LANG__:%s" % info.language, flush=True)

for segment in segments:
    line = "[%.2fs -> %.2fs] %s" % (segment.start, segment.end, segment.text)
    print("__SEG__:%.2f|%s" % (segment.end, segment.text.strip()), flush=True)
    with open(file_name, "a") as file:
        file.write(line + "\n")

print("__DONE__", flush=True)
