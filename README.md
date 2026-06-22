# ReqRadar

> Capture, inspecte, rejoue et compare tes requêtes HTTP — directement depuis le terminal.

ReqRadar est un outil en **Rust** pour intercepter le trafic HTTP, détecter les
patterns suspects (N+1, requêtes lentes, secrets qui fuient, retries en boucle…)
et transformer un bug réseau en rapport prêt à coller dans un ticket — le tout
sans quitter ton terminal, et avec un dashboard web optionnel.

> **Statut : phase zéro.** Le squelette du projet et la surface CLI sont en place.
> Le moteur de capture arrive dans les phases suivantes (voir la [roadmap](#roadmap)).

---

## Pourquoi ?

Déboguer une intégration HTTP aujourd'hui, c'est jongler entre `curl`, des logs
verbeux, un proxy lourd à configurer et des captures impossibles à rejouer.
ReqRadar vise le chemin le plus court entre « il y a un bug réseau » et « voici
exactement ce qui s'est passé, rejoue-le ».

## Aperçu de la CLI

```bash
reqradar                 # capture interactive (TUI)
reqradar capture --web   # capture + dashboard web (React/Vite)
reqradar replay <id>     # rejoue une requête capturée
reqradar diff <a> <b>    # compare deux captures (avant/après déploiement)
reqradar report <id>     # exporte un rapport de bug (Markdown/PDF)
reqradar rules check f.yml  # valide tes détecteurs custom (YAML)
```

> En phase zéro, ces commandes existent et sont documentées mais renvoient
> « not implemented yet ». La surface est figée pour construire le moteur
> derrière sans casser l'interface.

## Roadmap

La **phase zéro** (ce commit) pose les fondations : structure Cargo, surface CLI,
CI, licences et conventions.

### Cœur

- [ ] **Moteur de capture** — proxy HTTP/HTTPS qui enregistre requêtes et réponses
- [ ] **Stockage des captures** — format sur disque rejouable et diffable
- [ ] **Détecteurs de patterns** — N+1, lenteurs, statuts d'erreur, fuites de secrets

### Expérience

- [ ] **Mode hybride CLI/Web** — TUI terminal (ratatui) + `--web` qui lance un
      dashboard React/Vite avec graphes de patterns et arbre de requêtes
- [ ] **Replay** — rejoue une requête capturée en un clic/commande pour reproduire un bug
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
