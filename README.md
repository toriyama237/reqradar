# ReqRadar

> Capture, inspecte, rejoue et compare tes requêtes HTTP — directement depuis le terminal.

ReqRadar est un outil en **Rust** pour intercepter le trafic HTTP, détecter les
patterns suspects (N+1, requêtes lentes, secrets qui fuient, retries en boucle…)
et transformer un bug réseau en rapport prêt à coller dans un ticket — le tout
sans quitter ton terminal, et avec un dashboard web optionnel.

> **Statut : moteur de capture fonctionnel.** ReqRadar agit en reverse proxy
> transparent devant un backend local, enregistre chaque échange au format
> `.rrlog` (JSON Lines) et sait rejouer une requête capturée. La TUI, le
> dashboard web, le diff et le rapport arrivent ensuite (voir la [roadmap](#roadmap)).

---

## Pourquoi ?

Déboguer une intégration HTTP aujourd'hui, c'est jongler entre `curl`, des logs
verbeux, un proxy lourd à configurer et des captures impossibles à rejouer.
ReqRadar vise le chemin le plus court entre « il y a un bug réseau » et « voici
exactement ce qui s'est passé, rejoue-le ».

## Démarrage rapide

Place ReqRadar devant ton backend local et envoie ton trafic via le proxy :

```bash
# 1. ReqRadar écoute sur :8080 et forwarde tout vers ton backend sur :3000
reqradar capture --target http://localhost:3000 --listen 127.0.0.1:8080

# 2. Dans un autre terminal, tape le proxy au lieu du backend
curl http://localhost:8080/api/health
curl -X POST http://localhost:8080/api/login -d '{"user":"bob"}'
```

Chaque échange s'affiche en direct et s'écrit dans `captures/session-<ts>.rrlog` :

```
1782092105674-000001  GET    /api/health   ->  200  1ms
1782092105686-000002  POST   /api/login    ->  200  2ms
```

Puis rejoue n'importe quelle requête capturée par son id :

```bash
reqradar replay 1782092105686-000002          # utilise le dernier .rrlog de ./captures
reqradar replay <id> --file session.rrlog     # fichier explicite
reqradar replay <id> --target http://staging  # rejoue ailleurs
```

> Utilise `--json` sur `capture` pour cracher l'échange complet (headers + body)
> en JSON sur stdout.

## Aperçu de la CLI

```bash
reqradar capture --target <url>   # reverse proxy transparent + capture  ✅
reqradar replay <id>              # rejoue une requête capturée           ✅
reqradar capture --web            # capture + dashboard web (React/Vite)  ⏳
reqradar diff <a> <b>             # compare deux captures                 ⏳
reqradar report <id>              # exporte un rapport de bug (MD/PDF)     ⏳
reqradar rules check f.yml        # valide tes détecteurs custom (YAML)   ⏳
```

## Roadmap

La **phase zéro** a posé les fondations (structure Cargo, surface CLI, CI,
licences). La **phase un** ajoute le moteur de capture et le replay.

### Cœur

- [x] **Moteur de capture** — reverse proxy HTTP transparent qui enregistre
      requêtes et réponses (HTTPS/forward proxy à venir)
- [x] **Stockage des captures** — format `.rrlog` rejouable et diffable (JSON Lines)
- [ ] **Détecteurs de patterns** — N+1, lenteurs, statuts d'erreur, fuites de secrets

### Expérience

- [ ] **Mode hybride CLI/Web** — TUI terminal (ratatui) + `--web` qui lance un
      dashboard React/Vite avec graphes de patterns et arbre de requêtes
- [x] **Replay** — rejoue une requête capturée en un clic/commande pour reproduire un bug
- [ ] **Diff de requêtes** — compare deux captures (avant/après un déploiement) et
      montre ce qui a changé
- [ ] **Export « rapport de bug »** — génère un Markdown/PDF prêt à coller dans un
      ticket Jira/Linear avec tout le contexte
- [ ] **Règles custom en YAML** — chacun écrit ses propres détecteurs sans toucher
      au code Rust

### Distribution

- [ ] **`brew install reqradar`** + **`cargo install reqradar`** + **binaire statique
      téléchargeable** — zéro dépendance, zéro friction

## Développement

Prérequis : Rust stable (édition 2021).

```bash
cargo build            # compile
cargo run -- --help    # affiche l'aide
cargo test             # tests
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all
```

## Licence

Sous double licence, au choix :

- Apache License 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT ([LICENSE-MIT](LICENSE-MIT))

Sauf indication contraire, toute contribution que tu soumets est destinée à être
double-licenciée comme ci-dessus, sans condition supplémentaire.
