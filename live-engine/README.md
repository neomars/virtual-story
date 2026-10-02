# Live engine (Rust)

Remplace les boutons de choix par une conversation dynamique : une IA locale (non censurée selon le modèle
choisi) tient un personnage, parle (voix), t'écoute (micro) et **choisit elle-même** les vidéos/photos de la
médiathèque pour illustrer l'échange. Tout tourne en local, sur ton GPU NVIDIA.

```
App Ubuntu (Electron) ── lance ──> live-engine (Rust, axum, port 3001)
   │  Express :3000 (UI + admin) ── proxy /api/live ──┘   │
   │                                                      ├─ télécharge les modèles (Hugging Face, reprise + SHA-256)
   │                                                      ├─ lit l'architecture réelle du GGUF → plan VRAM
   │                                                      ├─ lance/arrête llama-server  (IA texte, GPU)   :8080
   │                                                      ├─ lance/arrête whisper-server (micro → texte)   :8081
   │                                                      └─ lance/arrête tts-server (Chatterbox, voix clonée) :8880
```

## Utilisation dans l'app

1. **Admin → Modèles IA** (`/admin/live/models`) : clique sur *Télécharger* (Gemma 4 12B Heretic recommandé, puis
   Whisper large-v3-turbo Q5, puis la voix Chatterbox après *Installer le moteur de voix*), puis *Charger*. L'écran
   affiche l'état, le plan mémoire, la VRAM réelle et les journaux. Les téléchargements reprennent après une coupure
   et sont vérifiés par SHA-256. Les derniers modèles choisis se rechargent au démarrage.
2. **Admin → Médiathèque** : *Scanner les uploads* (vidéos **et** photos), puis annote tags, ambiance, intensité.
3. **Admin → Personnages** : crée ton personnage (nom, personnalité, style, scénario, voix).
4. **Live** : choisis le personnage, parle ou écris. Parler pendant que l'IA répond la coupe (barge-in).

Ajouter un modèle au catalogue : crée `catalog.json` dans le dossier de données (liste d'objets
`{id, kind: "llm"|"stt"|"tts", name, repo, pattern | files, args, exec}`) ; un même `id` remplace l'entrée intégrée.
`HF_TOKEN` (variable d'environnement) permet de télécharger depuis des dépôts protégés.

## Comment l'IA choisit les médias

Le LLM écrit du texte normal avec des directives invisibles (plus fiable que le function-calling sur modèles locaux) :

| Directive | Effet |
|---|---|
| `[[show: plage, soleil \| mood=joyeux \| intensity=2 \| kind=photo]]` | affiche un média (jamais deux fois le même) |
| `[[ambient: appartement, soir \| mood=calme]]` | change la vidéo d'ambiance en boucle |
| `[[replies: Oui \| Non \| Plus tard]]` | boutons de réponse rapide (optionnels) |
| `[[state: confiance=3]]` | mémoire du récit, réinjectée à chaque tour |

L'IA ne voit que les tags réellement présents dans la médiathèque ; le plafond d'intensité (curseur 1-5) est appliqué
par le serveur.

## La voix de l'IA (Chatterbox)

Le moteur de voix est **Chatterbox multilingue** (licence MIT, français, 23 langues) : il **clone une voix à partir d'un
court échantillon** et règle l'expressivité, ce qui permet d'obtenir une voix féminine grave, soufflée ou posée selon
l'échantillon choisi. Les voix prédéfinies (Piper, Kokoro) n’offrent qu’une voix française neutre.

1. **Admin → Modèles IA → Voix de l'IA** : *Installer le moteur de voix* (environnement Python 3.11 isolé via `uv`,
   PyTorch/CUDA, ≈ 3 Go), puis *Télécharger* et *Charger* « Chatterbox multilingue » (≈ 4 Go de modèle).
2. **Admin → Personnages → Voix** : envoyez un échantillon (10 à 20 s, une seule personne, sans bruit ni musique, avec
   le ton recherché), choisissez-le comme voix de référence, réglez **Expressivité** (≈ 0,7 pour une voix intime) et
   **Rythme** (≈ 0,3 = plus lent et posé), puis *Écouter* pour ajuster. Le personnage « Camille » (25 ans) est fourni
   comme exemple ; sans échantillon, la voix par défaut du modèle est utilisée.
3. Le texte est synthétisé phrase par phrase pendant que l'IA écrit ; les gestes entre `*astérisques*` ne sont pas lus.

Utilisez votre propre voix ou un enregistrement dont vous détenez les droits (pas la voix d'une personne réelle sans son
accord). Le serveur est `scripts/tts/chatterbox_server.py` (API OpenAI `/v1/audio/speech`, champs en plus :
`exaggeration`, `cfg_weight`) ; `TTS_FAKE=1` le remplace par un simple bip pour tester sans GPU. Limite connue : cette
partie n'a été testée qu'avec le mode `TTS_FAKE` ; le premier lancement réel dépend de l'API de la version installée de
`chatterbox-tts` (les journaux de l'écran Modèles montrent l'erreur éventuelle).

## Adaptation à 15 Go de VRAM / 64 Go de RAM

Le plan est recalculé à chaque chargement depuis le **vrai** GGUF. Le cache KV est calculé comme le fait llama.cpp, couche
par couche : couches globales = `pad256(ctx)` cellules, couches à fenêtre glissante = `pad256(min(ctx, fenêtre + 512))`
cellules, octets = cellules × têtes KV × (dim K + dim V) × octets/élément (q8_0 = 1,0625). Pour Gemma 4 12B
(40 couches glissantes 8×256 + 8 globales 1×512, fenêtre 1024) cela donne ≈ **0,4 Go à 16k** de contexte, contre ≈ 3,4 Go
avec une attention complète. Le calcul est borné par la VRAM réellement libre (`nvidia-smi`) et réserve la place du
micro et de la voix **choisis mais pas encore chargés** (`hardware.vram_other_models_gb`, 5,7 Go par défaut = Whisper
≈ 1,2 Go + voix ≈ 4,5 Go). Si tout ne tient pas, le contexte baisse d'abord (jusqu'à 8k), puis quelques couches passent en RAM.

Budget indicatif (15 Go) : Gemma 4 12B Q4_K_M 7,4 Go + KV 0,4 + tampons 0,8 + Whisper 1,2 + Chatterbox ≈ 4,5 + OS 1 ≈ 15,3 Go →
1 à 2 couches en RAM. Pour tout garder sur le GPU : Whisper sur CPU (`stt.use_gpu = false`, `vram_other_models_gb = 4.5`) ou le
modèle Whisper « small ». `cargo run --release plan [id]` affiche le plan sans rien lancer.

| Profil | Modèle | Placement |
|---|---|---|
| **A (défaut)** | Gemma 4 12B Heretic Q4_K_M ≈ 7,4 Go, ctx 16k | GPU (voix + micro compris : quasi 100 %) |
| **B** | Cydonia 24B IQ4_XS ≈ 12,8 Go | GPU + RAM (voix sur le GPU ⇒ plus de couches en RAM) |

## Compiler l'app Ubuntu (NVIDIA)

Voir le README racine (`npm run doctor`, `npm run dist:linux`). Détails de `scripts/build-sidecars.sh` :
`llama-server` est le **binaire officiel CUDA 12.8** (tag `b11146`, vérifié par SHA-256 ; `LLAMA_CPP_TAG` / `LLAMA_SHA256` /
`CUDART_SHA256` pour une autre version, `LLAMA_MODE=source` pour le compiler) ; `whisper-server` est compilé
(`CUDA_ARCH=native` = la carte de la machine de build, ou par exemple `"86;89"` pour d'autres cartes ; `WHISPER_CPP_REF`,
`SKIP_WHISPER=1`) ; les scripts de voix sont copiés dans `bin/`. `FORCE=1` refait tout. Gemma 4 exige un llama.cpp récent
(juin 2026 ou après). Les versions installées sont notées dans `bin/VERSIONS.txt`. `scripts/verify-package.sh [dossier]`
contrôle un paquet déjà construit.

À l'exécution, l'app stocke tout dans ton dossier utilisateur : base et médias dans `~/.config/Virtual Story`,
modèles dans `~/.local/share/virtual-story/models` (`VS_MODELS_DIR` pour changer), moteur de voix Python dans
`~/.local/share/virtual-story/tts-venv`, voix de référence dans `<dossier de données>/voices`. Journaux : `~/.config/Virtual
Story/logs/live-engine.log`.

## Développement

```bash
cd live-engine && cargo run --release        # moteur sur :3001 (lit live.toml s'il existe)
cd frontend && npm run dev                   # Vite proxifie /api/live vers :3001
cargo test                                   # planificateur, GGUF, catalogue/téléchargement, directives, médias…
```

## Règles du moteur

- Usage personnel : aucune vérification de médias, aucun refus ni avertissement dans le prompt, curseur d'intensité à 5
  par défaut (réglage à toi).
- Une seule limite est conservée : les personnages sont des adultes. Une fiche avec `age < 18` est refusée à
  l'enregistrement et ignorée au chargement, et le prompt interdit tout contenu sexuel impliquant un mineur.
- Sécurité locale : le moteur écoute sur 127.0.0.1 ; les WebSocket venant d'un autre site web sont refusés ;
  `admin_token` protège l'édition si tu l'exposes sur un réseau.
