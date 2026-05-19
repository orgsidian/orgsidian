---
stepsCompleted: [1, 2, 3, 4]
inputDocuments: []
session_topic: 'Orgsidian — app desktop in stile Obsidian basata su org-mode invece di Markdown'
session_goals: 'Esplorare differenziazione rispetto a Obsidian, architettura tecnica candidata, target utenti (esperti org-mode/Emacs + nuovi arrivati che vogliono evitare la curva di Emacs). Progetto open-source, cross-platform, sviluppato in solo. Preferenza per ideazione convergente: poche idee solide e fattibili.'
selected_approach: 'progressive-flow'
techniques_used: ['What If Scenarios', 'Mind Mapping', 'First Principles Thinking', 'Resource Constraints']
ideas_generated: 35
session_active: false
workflow_completed: true
context_file: ''
---

# Brainstorming Session Results

**Facilitator:** Tiziano
**Date:** 2026-05-18

## Session Overview

**Topic:** Orgsidian — un'app desktop in stile Obsidian che usa org-mode invece di Markdown come formato nativo.

**Goals:**
- Capire come differenziarsi da Obsidian (e dagli editor org-mode esistenti)
- Esplorare opzioni architetturali concrete per il desktop cross-platform
- Definire target utenti: utenti org-mode/Emacs esistenti + nuovi utenti che vogliono org-mode senza la curva di Emacs
- Vincoli: open-source, single-developer, cross-platform (macOS/Linux/Windows), nessuna preferenza linguaggio
- Modalità: ideazione convergente — poche idee solide e fattibili

### Session Setup

Sessione esplorativa con orientamento alla fattibilità. L'utente vuole idee robuste, non un brainstorming "divergente puro". Il facilitatore filtrerà attivamente per fattibilità tecnica solo-sviluppatore.

## Technique Selection

**Approach:** Progressive Technique Flow
**Journey Design:** Esplorazione → Pattern Recognition → Sviluppo dai principi primi → Piano d'azione vincolato

**Progressive Techniques:**

- **Phase 1 — Exploration:** What If Scenarios per generare possibilità di differenziazione + opzioni architetturali
- **Phase 2 — Pattern Recognition:** Mind Mapping per clusterizzare in temi e selezionare i finalisti
- **Phase 3 — Development:** First Principles Thinking per validare i concetti finalisti dai fondamentali
- **Phase 4 — Action Planning:** Resource Constraints per arrivare a un MVP fattibile per un solo developer

**Journey Rationale:** Per un prodotto tecnico open-source mono-sviluppatore servono idee solide. Si esplora ampio per non perdere differenziatori; si converge per principi primi per evitare scelte di moda; si vincola alla realtà di un solo developer per ottenere un piano eseguibile.

---

## Technique Execution Results

### Phase 1 — What If Scenarios (Exploration)

**Interactive Focus:** Differenziazione vs Obsidian, opzioni architetturali, posizionamento di prodotto, esperienza di onboarding, storage/sync.

**Key Breakthroughs:**
- Riconoscimento che org-mode è fondamentalmente più orientato a task/time che a note → apre un posizionamento competitivo non saturato
- Pattern "Smart Defaults, User in Control" emerso trasversalmente come principio cardine del prodotto
- Decisione netta su filesystem-native + indice SQLite (modello org-roam) come spina dorsale dello storage
- Cancellazione esplicita di scope creep pericolosi: no Emacs backend, no CRDT real-time, no sync server in v1

**User Creative Strengths:** Decisioni nette e veloci. Buon "naso" per la complessità (es. ha bloccato l'engine standalone perché "complesso da mostrare", proponendo subito una terza via — monorepo con package separati).

**Energy Level:** Alta, costante. Risposte rapide, scelte chiare, niente esitazioni.

### Phase 2 — Mind Mapping (Pattern Recognition)

**Building on Previous:** 22 idee della Fase 1 clusterizzate in 6 temi coerenti (Architettura, Posizionamento, UX/Editor, Onboarding, Storage&Sync, Principi). Sono emerse 4 connessioni inter-cluster importanti, in particolare la triade "task-first positioning ⇄ agenda home ⇄ today dashboard" che mostra come una decisione di UX equivalga a una dichiarazione di posizionamento.

**New Insights:** Identificati 7 buchi nella mappa, di cui 4 risolti subito con decisioni chiare (plugin: sì estendibile, search: full-text + semantic + backlinks, quick capture: must-have, theme: personalizzabile) e 3 differiti (stack: a architetto, mobile: out-of-scope, AI/LLM: post-MVP).

**Developed Ideas:** Plugin/Extensibility System, Quick Capture nativo, Search a 3 livelli, Themes personalizzabili → 4 nuove idee strutturate.

### Phase 3 — First Principles Thinking (Validation)

**Building on Previous:** Validati i 3 concetti più portanti (posizionamento, architettura, onboarding) dai fondamentali. Tutti e tre hanno retto, ma con raffinamenti significativi.

**Candidato A — Posizionamento:** Riconosciuto che "task-first" è un wedge di marketing, non architettura di prodotto. Le tre dimensioni (task, time, knowledge) sono **peer features unificate**, e questa unificazione è il vero differenziatore difendibile.

**Candidato B — Architettura:** Sono emerse 3 assunzioni nascoste pericolose: (1) "scrivere il parser da zero" → invece riusare uniorg/tree-sitter; (2) "rimandare la scelta dello stack" → impossibile, decisione cascata su tutto; (3) "file watcher banale" → in realtà punto fragile che richiede Single Writer Rule + Dirty Buffer. Aggiunte 3 decisioni architetturali critiche.

**Candidato C — Onboarding:** Insight più importante della sessione: **il vero blocco non è la sintassi ma il workflow**. Le 4 idee originali erano 75% syntax-focused. Aggiunte 3 idee workflow-focused (Starter Vault precompilato, Workflow Recipes, Inline Coaching) e riformulato il Tutorial in chiave workflow-first.

**Developed Ideas:** 8 nuove idee/decisioni strutturate (Architettura raffinata, Decisioni architettura #3/#4/#5, Onboarding #5/#6/#7, Raffinamento Onboarding #3).

### Phase 4 — Resource Constraints (Action Planning)

**Building on Previous:** Applicati i vincoli reali (10h/settimana, single dev, OSS, no budget, no deadline ma puntiamo a qualità). La matematica brutale ha rivelato che Option C completo costa 1250-1850h (~3-4.5 anni) — necessario rilascio incrementale.

**Critical Trade-off Identified:** WYSIWYG vero (ProseMirror) costa 240-320h, ~1/5 del budget totale. Soluzione: pseudo-WYSIWYG via syntax highlighting evoluto in CodeMirror 6 → costo 60-80h.

**Roadmap consolidata:** Pre-MVP (mesi 1-2: spike) → v0.1 Alpha (mesi 3-6: read-only navigator) → v0.5 Beta (mesi 7-12: daily driver + Project Report) → v1.0 (mesi 13-18: tutorial + Windows + polishing). WYSIWYG vero e Plugin pubblica rimandati a v1.5+.

**User Decisions:**
- WYSIWYG = Opzione Z (pseudo-WYSIWYG)
- Roadmap incrementale alpha→beta→1.0 confermata
- Time tracking spostato da v0.5 a v1.0 (richiede UX curata)
- Project Report anticipato da v1.0 a v0.5 (wow demo per il lancio pubblico)
- Stack: preferenza Rust, decisione finale con architetto

**Overall Creative Journey:** La sessione ha trasformato un'idea generica ("app come Obsidian ma con org-mode") in un prodotto preciso con posizionamento, architettura, MVP scope e roadmap a 18 mesi. La progressione divergente→convergente ha funzionato: si è esplorato ampio in Fase 1 senza perdere ore in dettagli, si è convergiti con disciplina in Fase 3, e si è atterrato in Fase 4 con un piano realistico.

### Creative Facilitation Narrative

Sessione efficiente e densa. L'utente è arrivato con una visione vaga ma volontà chiara di "esplorare con orientamento alla fattibilità". Il facilitatore ha bilanciato esplorazione e convergenza non perdendo tempo su idee chiaramente fuori scope (es. Emacs backend, CRDT real-time), e investendo invece su raffinamenti di precisione (es. la distinzione syntax-vs-workflow in onboarding, il trade-off WYSIWYG). L'utente ha contribuito con istinto pragmatico — il momento più creativo è stato la proposta della "terza via" monorepo (engine + UI in due package deployabili separatamente), nato come reazione spontanea a un trade-off architetturale.

### Session Highlights

**User Creative Strengths:** Decisioni rapide e ben argomentate; istinto solido per scoping (capisce subito quando un'idea è "troppo" e propone compromessi); apertura a cambiare mente su raffinamenti (es. ha accettato l'inversione "task-first marketing, integrazione prodotto"); buona resilienza ai dettagli tecnici complessi.

**AI Facilitation Approach:** Provocazioni "What If" puntate sul punto di tensione (non genericamente "creative"), follow-up con tradeoff pros/cons concreti, framing di principi primi che hanno smascherato assunzioni inconsce, matematica trasparente in Fase 4 senza addolcire.

**Breakthrough Moments:**
1. "Monorepo Core+Shell come terza via" (Fase 1) — risolve l'apparente dicotomia engine-vs-monolite
2. "Task & Project Planner FIRST" come posizionamento (Fase 1) — sblocca un mercato non saturo
3. "Filesystem-native + SQLite index" come architettura storage (Fase 1) — pattern collaudato da org-roam
4. "Smart Defaults, User in Control" come principio cardine (Fase 1) — emerge trasversalmente
5. "Onboarding è workflow, non syntax" (Fase 3) — capovolge la strategia di onboarding originale
6. "Pseudo-WYSIWYG via syntax highlighting" (Fase 4) — risolve 1/5 del budget totale

**Energy Flow:** Costantemente alta. Nessun "tracollo di attenzione". L'utente ha guidato il ritmo (es. "next" quando la mappa era satura) — segno di brainstorming sano.

---

## Idea Organization and Prioritization

### Thematic Organization (35 idee finali)

#### 🏗 Tema 1 — Architettura & Tecnologia (10 idee)

- **[Architettura #1]: Monorepo "Core + Shell"** — engine package + desktop package, in-process per evitare IPC ma confine API pulito
- **[Decisione #1]: Zero dipendenza da Emacs** — parser org-mode nativo, no batch mode Emacs
- **[Decisione architettura #3]: Non riscrivere il parser** — usare uniorg / tree-sitter-org + layer semantico custom
- **[Decisione architettura #4]: Single Writer Rule + Dirty Buffer** — solo l'editor scrive sui file, modifiche esterne ricaricano con merge dialog
- **[Decisione architettura #5]: Plugin pattern interno fin dalla v1** — codice interno usa già hooks/registry, esposizione pubblica in v1.5+
- **[Storage #1]: Filesystem-native** — `.org` files come source of truth, sync via Git/Syncthing/iCloud
- **[Storage #2]: SQLite come indice** — alla org-roam, cache rigenerabile dai file
- **[Decisione #2]: NO CRDT/multi-device sync built-in** — scope ridotto, Git+Syncthing bastano
- **[Open question architettura]: CodeMirror 6 compatibility con stack** — da validare in spike Mese 1-2 (funziona in Electron e in Tauri via webview)
- **[Stack]: preferenza Rust (Tauri) per performance, decisione finale con architetto**

#### 🎯 Tema 2 — Posizionamento & Target (3 idee)

- **[Posizionamento #1]: "Task & Project Planner Powered by Org-mode"** — non "Obsidian-with-org", ma planner+KM integrato. One-liner: *"The integrated planner & knowledge tool — built on org-mode, no Emacs required."*
- **[Raffinamento posizionamento]: Marketing task-first, prodotto integrazione** — task come wedge di marketing per entrare in un mercato non saturo, ma il prodotto è integrazione peer-level di task/time/knowledge
- **[Target #1]: Audience ampia, freelancer come persona faro** — target generale = knowledge worker; freelancer/consulente è la persona chiave da servire bene

#### 🖥 Tema 3 — UX / Editor (6 idee)

- **[UX #1]: Agenda-First Home View** — apertura su "cosa fare oggi", non file list o grafo
- **[UX #2]: Time-Tracking come barra persistente (toggleable)** — visibile e prima classe, ma nascondibile
- **[UX #3]: "Today" Dashboard iniettato** — daily note auto-popolata con agenda + clocked tasks + inbox + log settimanale
- **[UX #4]: Keybindings desktop-native (non Emacs)** — Cmd/Ctrl + lettera singola; modalità "Emacs keybindings" opzionale per power user
- **[Onboarding #1] (Editor modes)**: Raw / WYSIWYG / Split selezionabili dall'utente
- **[Decisione MVP #1]: WYSIWYG = Opzione Z (pseudo-WYSIWYG)** — syntax highlighting evoluto su CodeMirror, no ProseMirror in v1

#### 🎓 Tema 4 — Onboarding & Adozione (7 idee)

- **[Onboarding #2]: Plain Mode → Power Mode** — progressive disclosure delle feature avanzate
- **[Onboarding #3 raffinato]: Interactive Tutorial workflow-first** — 10 min, fa lavoro vero (capture → task → schedule → agenda → clock → report), non insegna sintassi
- **[Onboarding #4]: Side-by-side Source/Render (opzionale)** — vista split per imparare la sintassi guardando il rendering
- **[Onboarding #5]: Starter Vault precompilato** — 4 template (Personal GTD, Student, Freelancer, Empty) con file e agenda popolata
- **[Onboarding #6]: Workflow Recipes (post-MVP, v1.5+)** — gallery di workflow pre-pacchettati (GTD, PARA, Zettelkasten, Weekly Review, OKR)
- **[Onboarding #7]: Inline Coaching** — empty states con suggerimenti contestuali, command palette descrittiva
- **[Principio #1]: Smart Defaults, User in Control** — ogni feature opinionated ha default sensato + toggle

#### 🚀 Tema 5 — Features (5 idee)

- **[Feature #1]: Project Report Export (1-click)** — PDF/HTML con task done, ore tracciate, note collegate, milestone
- **[Feature #2 / candidate]: Git integration nativa** — history, diff, branch per scenari di pianificazione
- **[Feature #3]: Plugin/Extensibility System** — supportato fin dall'architettura, esposto pubblicamente in v1.5+
- **[Feature #4]: Quick Capture (org-capture nativo)** — hotkey globale OS-level, system tray
- **[Feature #5]: Search a 3 livelli** — full-text + semantic (post-MVP) + backlinks-aware
- **[Feature #6]: Themes personalizzabili** — CSS-based, dark+light di default

#### 📌 Tema 6 — Scope & Backlog (4 decisioni)

- **[Scope-out] Mobile** — out of scope v1.0
- **[Scope-out] AI/LLM** — post-MVP, hook architetturale predisposto
- **[Backlog #1] Sync server self-hostable** — post-MVP, alternativa OSS a Obsidian Sync
- **[Roadmap swap finale]:** Project Report alla v0.5 Beta (wow demo lancio pubblico), Time Tracking alla v1.0 (richiede UX curata)

### Prioritization Results

**Top Priority Ideas (foundational — MVP-critical):**

1. **Architettura "Core + Shell + Filesystem + SQLite Index"** — spina dorsale del prodotto, condiziona tutto il resto
2. **Posizionamento "Task & Project Planner Powered by Org-mode"** — il claim che differenzia in mercato saturo
3. **Quick Capture nativo** — feature più amata di org-mode, must-have psicologico per ex-Emacs user
4. **Agenda-First Home + Today Dashboard** — comunicano subito il posizionamento, "wow" del primo avvio
5. **Starter Vault precompilato** — riduce attrito immediato e insegna il workflow

**Quick Win Opportunities (basso costo, alto ritorno):**

- Themes dark+light minimalisti (40h)
- Plain Mode → Power Mode UI rules (40h)
- Project Report export (40-80h, killer feature per il marketing del v0.5 lancio)

**Breakthrough Concepts (longer-term, da pianificare ora ma costruire dopo):**

- WYSIWYG vero (ProseMirror + Org schema) — v1.5+
- Plugin API pubblica — v1.5+
- Workflow Recipes gallery — v1.5+
- Sync server self-hostable — v2+

### Action Planning

#### Idea Priority 1 — Lock Architettura & Stack

**Why This Matters:** È la decisione che cascade su tutto. Sbagliarla qui = riscrittura mostruosa dopo.

**Next Steps:**
1. Sessione con BMad architect (`/bmad-create-architecture`) per decidere stack definitivo (Electron+TS vs Tauri+Rust)
2. Spike 1: prototipo "open + parse + render" minimal con stack scelto (~30h)
3. Spike 2: file watcher + Single Writer Rule cross-platform (~20h)
4. Spike 3: SQLite index su vault di test da 1000 file, misurare query agenda (~10h)
5. Decisione plugin pattern interno: registry + hooks + event bus (~20h design)

**Resources Needed:** Tempo per spike (60-80h totali in Mese 1-2). Nessun costo monetario.
**Timeline:** Mesi 1-2 (~80h budget)
**Success Indicators:** Decisione stack pubblicata; prototipo end-to-end funzionante su un file `.org`; benchmark SQLite documentato.

#### Idea Priority 2 — Costruire v0.1 Alpha (read-only navigator)

**Why This Matters:** Il primo rilascio pubblico. Crea momentum, attira feedback, valida ipotesi di posizionamento prima di investire 18 mesi.

**Next Steps:**
1. Parser wrapper + AST stabile (80-120h)
2. Editor Raw mode con CodeMirror 6 + Single Writer Rule (60-80h)
3. Agenda view base today/week (80-120h)
4. Theming dark+light (30-40h)
5. Packaging Mac+Linux + auto-update base (40-60h)
6. Sito landing minimal + repo pubblico ben curato

**Resources Needed:** ~160h di sviluppo. Domain pubblico (~10$/anno).
**Timeline:** Mesi 3-6 (~160h budget)
**Success Indicators:** Annuncio su HackerNews/Reddit r/orgmode con almeno 50 commenti tecnici; 10+ early adopter che lo provano sui loro vault esistenti.

#### Idea Priority 3 — Costruire v0.5 Beta (daily driver + wow demo)

**Why This Matters:** Il punto in cui Orgsidian smette di essere "prototipo" e diventa "tool". Validation key: tu stesso lo usi al posto di Emacs/Obsidian?

**Next Steps:**
1. Quick Capture nativo (60-100h)
2. Today Dashboard (40-60h)
3. Plain Mode → Power Mode + Settings UI (30-50h)
4. Inline Coaching + command palette descrittiva (40-80h)
5. Starter vault templates (Personal GTD + Student) — 40h
6. Search FTS5 + backlinks UI (60-100h)
7. Pseudo-WYSIWYG via CodeMirror 6 syntax highlighting (60-80h)
8. **Project Report export — wow demo per il lancio v0.5** (40-80h)

**Resources Needed:** ~280h. Eventualmente community Discord/Matrix per beta tester.
**Timeline:** Mesi 7-12 (~240h budget — possibile lieve overflow, accettabile)
**Success Indicators:** Tu lo usi 5 giorni/settimana; 100+ beta tester attivi; segnalazioni bug ricorrenti su 3-5 aree (vuol dire che il prodotto è usato sul serio).

#### Idea Priority 4 — Costruire v1.0 (public launch)

**Why This Matters:** L'annuncio "ufficiale". Da qui si misura la reach del progetto.

**Next Steps:**
1. Time Tracking bar + clocking persistente (40h)
2. Interactive Tutorial workflow-first (60-100h)
3. Starter vault completi (Freelancer + Empty oltre i due esistenti) — 40h
4. Packaging Windows + auto-update completo (40-60h)
5. Polishing pesante: perf, edge case, UX detail (60h+)
6. Sito web + documentazione + changelog
7. Annuncio coordinato: HackerNews, ProductHunt, blog org-mode community

**Resources Needed:** ~280h. Eventualmente design help per landing page (richiesta volontaria nella community).
**Timeline:** Mesi 13-18 (~240h budget)
**Success Indicators:** 1000+ download nei primi 30 giorni; copertura su 1-2 newsletter org-mode/productivity; primi 3-5 contributor esterni che mandano PR.

---

## Session Summary and Insights

### Key Achievements

- **35 idee strutturate** organizzate in 6 cluster tematici coerenti
- **Posizionamento del prodotto chiaro** — non "Obsidian-with-org", ma "integrated planner & knowledge tool"
- **Architettura tecnica definita** a livello di componenti e pattern (Core+Shell, filesystem+SQLite, Single Writer Rule, plugin pattern interno)
- **Strategia onboarding ridisegnata** dai principi primi — da syntax-focused a workflow-focused
- **Roadmap a 18 mesi** in 4 milestone (Pre-MVP / Alpha / Beta / v1.0) realisticamente eseguibile a 10h/settimana
- **6 decisioni di scope-out** chiare (no Emacs backend, no CRDT, no mobile, no AI/LLM in MVP, ecc.) che proteggono il progetto da scope creep

### Session Reflections

**Cosa ha funzionato:**
- Convergent technique selection (Progressive Flow) ha rispettato la preferenza dell'utente per "poche idee solide"
- L'analisi First Principles è stata la fase a più alto valore aggiunto — ha rivelato 3 assunzioni nascoste pericolose che avrebbero causato problemi in implementazione
- La matematica trasparente di Fase 4 ha disinnescato l'ottimismo non realistico e portato a un piano credibile
- L'utente ha guidato il ritmo (segnali "next" tempestivi) — ha permesso di restare convergenti senza forzare profondità inutile

**Lezioni chiave per il follow-up:**
- Il singolo trade-off architetturale più importante (Electron vs Tauri) richiede uno spike concreto, non una decisione astratta
- Onboarding workflow-first è probabilmente la decisione di prodotto più sottovalutata — vale la pena renderla esplicita nel PRD
- Il pattern "Smart Defaults, User in Control" merita essere principio di design documentato (riapparirà in decine di decisioni UX micro)
- Il rilascio incrementale (Alpha → Beta → 1.0) richiede disciplina nel non aggiungere feature in corsa — usare il documento di sessione come anchor

### Next BMad Steps (consigliati)

1. **`/bmad-create-architecture`** — sessione con architetto per definire stack tecnico (Electron+TS vs Tauri+Rust), validare componenti, decidere parser library (uniorg vs tree-sitter-org), e definire il plugin pattern interno
2. **`/bmad-prd`** — Product Requirements Document basato su questa sessione, con focus su posizionamento, MVP scope (v0.1 Alpha + v0.5 Beta), e success metrics
3. **`/bmad-create-epics-and-stories`** — dopo PRD + Architecture, decomporre v0.1 Alpha in epic e story implementabili
4. **`/bmad-product-brief`** (opzionale) — se ti serve un brief più narrativo da condividere con potenziali contributor/feedback givers

