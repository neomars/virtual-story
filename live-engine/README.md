# Live engine (Rust)

Remplace les boutons de choix par une conversation dynamique : une IA locale (non censurée selon le modèle
choisi) tient un personnage, parle (voix), t'écoute (micro) et **choisit elle-même** les vidéos/photos de la
médiathèque pour illustrer l'échange.

```
Navigateur (Vue /live) ──WebSocket──> live-engine (Rust, axum)
                                       ├─ llama-server  (LLM, GPU)      :8080
                                       ├─ whisper-server (micro → texte) :8081
                                       ├─ serveur TTS OpenAI-compatible  :8880  (Kokoro, CPU/GPU léger)
                                       └─ SQLite : médias annotés + personas JSON
```

## Comment l'IA choisit les médias

Le LLM écrit du texte normal avec des directives invisibles (plus fiable que le function-calling sur modèles locaux) :

| Directive | Effet |
|---|---|
| `[[show: plage, soleil \| mood=joyeux \| intensity=2 \| kind=photo]]` | affiche un média (jamais deux fois le même) |
| `[[ambient: appartement, soir \| mood=calme]]` | change la vidéo d'ambiance en boucle |
| `[[replies: Oui \| Non \| Plus tard]]` | boutons de réponse rapide (optionnels) |
| `[[state: confiance=3]]` | mémoire du récit, réinjectée à chaque tour |

Le moteur ne lui montre que **les tags réellement présents** dans la médiathèque et applique côté serveur
le plafond d'intensité choisi par l'utilisateur.

## Adaptation à 64 Go de RAM / 15 Go de VRAM

`live-engine plan` calcule le budget (poids + cache KV q8_0 + tampons) et les arguments de `llama-server` :

| Profil | Modèle | VRAM | Vitesse | Config |
|---|---|---|---|---|
| **A (défaut)** | 12B (base Mistral-Nemo) Q5_K_M ≈ 8,7 Go, ctx 16k | ≈ 11 Go, 100 % GPU | rapide | `live.example.toml` |
| **B** | 24-32B Q4_K_M ≈ 14-19 Go, ctx 12k | 13 Go + reste en RAM | plus lent (offload partiel) | décommenter le profil B |

Reste de la VRAM (≈ 1-2 Go) : Whisper `small` sur GPU. Le TTS (Kokoro) tourne sur CPU sans gêner le LLM.
Choisis le fine-tune GGUF que tu veux (le moteur parle l'API OpenAI : Ollama/vLLM fonctionnent aussi).
Les vitesses réelles dépendent de ta carte : lance `cargo run --release plan` puis mesure.

## Démarrage

```bash
cd live-engine
cp live.example.toml live.toml
cargo run --release plan                 # plan mémoire + commande llama-server
llama-server <args du plan> &            # ou llm.autostart = true
whisper-server -m ggml-small.bin -l fr --port 8081 &
# + un serveur TTS compatible /v1/audio/speech sur :8880
cargo run --release                      # http://127.0.0.1:3001
cd ../frontend && npm run dev            # /live, /admin/live/media, /admin/live/personas
```

1. `/admin/live/media` → « Scanner les uploads » (vidéos **et photos**), puis annote : tags, ambiance, intensité.
2. `/admin/live/personas` → crée ton personnage (nom, personnalité, style, scénario, voix).
3. `/live` → choisis le personnage, parle ou écris. Parler pendant que l'IA répond la coupe (barge-in).

## Profil retenu : Gemma 4 12B « Heretic » (Q4_K_M)

Config prête à l'emploi : `live.gemma4-12b.toml` (plan : tout sur GPU, ≈ 11,4 Go de VRAM estimés avec un cache KV
volontairement surestimé, contexte 16k).

```bash
cd live-engine
cp live.gemma4-12b.toml live.toml
pip install -U "huggingface_hub[cli]"
huggingface-cli download igorls/gemma-4-12B-it-heretic-GGUF --include "*Q4_K_M*" --local-dir models
# ajuste llm.model_path dans live.toml au nom exact du fichier téléchargé
cargo run --release plan                 # affiche la commande llama-server à lancer
```

Points d'attention :
- Gemma 4 demande un **llama.cpp récent** (compilé après le 2026-06-04). Compile-le avec ton backend GPU
  (`-DGGML_CUDA=ON` pour NVIDIA, `-DGGML_VULKAN=ON` pour AMD/autre).
- Le mode « thinking » est coupé par `--chat-template-kwargs '{"enable_thinking":false}'` ; s'il apparaît quand même
  dans les réponses, mets à jour llama.cpp.
- Si les réponses contiennent des `---` répétés, le chat template du GGUF est défectueux : relance avec `--jinja`
  et le template officiel du modèle de base.
- `n_kv_heads` et `head_dim` sont des estimations (voir le commentaire dans le fichier) : à vérifier dans `config.json`.
- À tester dès le premier lancement : le respect des balises `[[show: …]]` / `[[replies: …]]` sur 30-50 tours. Si le
  modèle les oublie ou les déforme, compare avec `Ministral-3-14B-Nymphaea-RP` ou `Rocinante-X-12B`.

## Règles du moteur

- Usage personnel : aucune vérification de médias, aucun refus ni avertissement dans le prompt, et le curseur
  d'intensité (1-5) est à 5 par défaut ; c'est un réglage à toi, pas une restriction.
- Une seule limite est conservée : les personnages sont des adultes. Une fiche avec `age < 18` est refusée à
  l'enregistrement et ignorée au chargement, et le prompt interdit tout contenu sexuel impliquant un mineur.
- Le serveur écoute sur 127.0.0.1 ; `admin_token` protège l'édition si tu l'exposes sur le réseau.

## Tests

`cargo test` : planificateur matériel, parseur de directives en flux, recherche de médias, SSE, personas.
