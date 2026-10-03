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
`exaggeration`, `cfg_weight`) ; `TTS_FAKE=1` le remplace par un simple bip pour tester sans GPU. Le filigrane audio inaudible de
Chatterbox (`perth`) importe `pkg_resources`, absent des `setuptools` récents : l'installation fixe donc `setuptools<81` et vérifie `perth`
(version d'installation 2 ; une installation plus ancienne affiche « Mettre à jour le moteur de voix »). Si le chargement échoue, les journaux
de l'écran Modèles montrent l'erreur.

## Adaptation à ta carte (VRAM détectée) et à ta RAM

Le plan est recalculé à chaque chargement depuis le **vrai** GGUF. Le cache KV est calculé comme le fait llama.cpp, couche
par couche : couches globales = `pad256(ctx)` cellules, couches à fenêtre glissante = `pad256(min(ctx, fenêtre + 512))`
cellules, octets = cellules × têtes KV × (dim K + dim V) × octets/élément (q8_0 = 1,0625). Pour Gemma 4 12B
(40 couches glissantes 8×256 + 8 globales 1×512, fenêtre 1024) cela donne ≈ **0,4 Go à 16k** de contexte, contre ≈ 3,4 Go
avec une attention complète. Le calcul est borné par la VRAM réellement libre (`nvidia-smi`) et réserve la place du
micro et de la voix **choisis mais pas encore chargés** (`hardware.vram_other_models_gb`, 5,7 Go par défaut = Whisper
≈ 1,2 Go + voix ≈ 4,5 Go). Si tout ne tient pas, le contexte baisse (jusqu'à 8k) seulement quand cela libère au moins une couche de poids ; sinon quelques couches passent en RAM (génération un peu plus lente).

La VRAM est **détectée automatiquement** (`hardware.vram_gb = 0`) : une RTX 4000 Ada portable, par exemple, a 12 Go et non 15.
L'écran Modèles affiche, pour chaque serveur, la mémoire GPU réellement utilisée (lue avec `nvidia-smi`).

### Choisir la taille de l'IA selon la carte (voix et micro chargés)

Ordre de grandeur sur 12 Go : voix Chatterbox ≈ 4,5 Go + Whisper « small » ≈ 0,5 Go (ou « large-v3-turbo » ≈ 1,2 Go) ⇒ il reste
≈ 6 Go pour l'IA (poids + cache + tampons ≈ 1,2 Go).

| Groupe | Modèle | Fichier | VRAM | Remarque |
|---|---|---|---|---|
| Mini | Qwen 3.5 4B non censuré | ≈ 2,6 Go | ≈ 4 Go | refus 0/465 annoncés ; français non mesuré |
| Mini | Ministral 3 3B Heresy | ≈ 2,1 Go | ≈ 3,3 Go | orienté jeu de rôle ; non mesuré |
| Mini | Gemma 4 E2B abliteré | ≈ 3,4 Go | ≈ 4,6 Go | français plus faible que E4B |
| **Léger** | **Gemma 4 E4B Heretic** | ≈ 5,0 Go | ≈ 6,2 Go | **recommandé avec la voix sur 12 Go** |
| Léger | Gemma 4 E4B abliteré / RP | ≈ 5 Go | ≈ 6,3 Go | variantes (RP : français à tester) |
| Léger | Gemma 4 12B Heretic IQ3_XS | ≈ 5,4 Go | ≈ 6,6 Go | le 12B compressé : perte de qualité visible |
| Léger | Qwen 3.5 9B non censuré | ≈ 5,3 Go | ≈ 6,5 Go | à la limite sur 12 Go avec la voix |
| Standard | Gemma 4 12B Heretic Q4_K_M | ≈ 7,4 Go | ≈ 8,6 Go | idéal sans la voix sur le GPU, ou avec une carte ≥ 16 Go |

Tous les modèles de texte « Léger » et « Mini » ci-dessus sont des modèles non censurés publiés sur Hugging Face ; les chiffres de refus viennent des
auteurs et le français n'a été mesuré sur aucun d'eux : essaie-en deux ou trois avec ton personnage. Les modèles Gemma/Qwen ont un mode « réflexion » : il est
coupé au chargement (`--reasoning off`). Ajoute les tiens dans `catalog.json`.

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
