import requests, sys
TOKEN = "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"
ARTIFACT_ID = sys.argv[1] if len(sys.argv) > 1 else "7975422861"
OUT = sys.argv[2] if len(sys.argv) > 2 else "/tmp/mlog-new.zip"
r = requests.get(
    f"https://api.github.com/repos/ShkodnikAI/Metalogos-/actions/artifacts/{ARTIFACT_ID}/zip",
    headers={"Authorization": f"Bearer {TOKEN}"},
    allow_redirects=True
)
with open(OUT, "wb") as f:
    f.write(r.content)
print(f"Downloaded {len(r.content)} bytes to {OUT}")