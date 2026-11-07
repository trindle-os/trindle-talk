# Talk help

## What Talk does

Talk lets you speak instead of typing. You press its icon in the Trindle ring, say what you want to write, and pause. A moment later your words appear under Talk's name in the ring. Talk can leave out ums and ers, use British spelling, and read your words back to you in English.

Talk has no window of its own. Its settings are on the Your plugins page.

## Starting it

1. Press Talk in the ring.
2. Wait for the "Listening" message, then speak.
3. Pause. Talk stops about a second and a half after you stop speaking.
4. Read your words under Talk's name in the ring.

The first time, Talk has to fetch its speech recognition (about 293 MB). That press only downloads. When the status says "Ready", press Talk again and speak. If you turn on Read back, the reading voice (about 264 MB) is fetched the same way, once. After that Talk works without the network.

Talk gives up if nobody speaks within 6 seconds, and a single recording stops at 10 minutes. Anything under half a second is ignored. While it works Talk needs about 1 GB of free memory.

## Settings

| Setting | Default | What it changes |
|---|---|---|
| Language you speak | Any language | The language Talk listens for. Any language lets it work out which one you are speaking. There are 19 to choose from. |
| Leave out um and er | On | Takes um, er, erm, uh, hmm and similar hesitations out of what was heard, and tidies the commas left behind. Works for English, or when the language is Any language. |
| British spelling | On | Changes American spellings to British ones, for example color to colour. Works for English, or when the language is Any language. |
| Read back what Talk heard | Off | Says your words aloud once they are heard. English only. |
| Reading voice | Male 5 | Which of the ten voices reads back (Female 1 to 5, Male 1 to 5). It only shows when Read back is on. |

The first line on the settings page is a status note. It shows what Talk is doing now, such as "Listening." or "Ready. Press Talk in the ring and speak."

## If something goes wrong

| What you see | What it means | What to do |
|---|---|---|
| Talk needs the microphone. Allow it in Settings, Permissions. | Talk has not been given the microphone, or no microphone is available. | Allow the microphone for Talk in Settings, Permissions, then press Talk again. |
| Could not fetch the speech recognition (or the reading voice). Allow the network in Settings, Permissions, then press Talk again. | The one-off download did not work. | Allow the network for Talk, check you are online, then press Talk again. |
| Voice is not set up. Get it from the Voice page in Settings. | The speech files are not on this computer yet. | Open the Voice page in Settings and get them, or press Talk again once the download has finished. |
| Voice is not set up. Talk needs the microphone and sound permissions. | The voice program could not start. | Allow the microphone and sound for Talk, then press Talk again. |
| Voice is not running. | The voice program stopped or could not be reached. | Press Talk again. If it keeps happening, restart Trindle. |
| Voice stopped, sometimes followed by a reason | The voice program hit a problem while listening or reading. | Press Talk again, and include the reason if you report it. |
| Talk heard nothing | No speech was picked up, or it was too short. | Press Talk and speak a little louder or closer to the microphone. |
| Talk cannot read back | Read back is on but your language is not English. | Set the language to English or Any language, or turn Read back off. |

If your words come out wrong, choose your language in the settings rather than leaving it on Any language. Speech recognition makes mistakes, so check what Talk heard before you rely on it.

## Privacy

- Listening and reading back both happen on this computer. Nothing you say is sent anywhere.
- Your voice is processed as it comes in and the audio is never written to disk. The words Talk heard are kept as lines in notify.jsonl in the plugin folder, and the oldest are trimmed.
- The network is used only to fetch the speech files, from Hugging Face, and each file is checked against a fixed checksum before it is kept.
- Talk keeps its settings and the latest status in its own plugin folder. The speech files are kept in `~/.local/share/trindle/voice`.
- Talk asks for four permissions and you choose which to allow: the microphone (to hear you), sound (to read back), the network (to fetch the speech files once) and notifications (to show your words).
