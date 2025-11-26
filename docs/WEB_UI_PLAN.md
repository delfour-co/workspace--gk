# 🎨 Plan Web UI - Interface AI-Native

**Date**: 2025-11-20
**Durée estimée**: 3-4 semaines
**Objectif**: MVP conversationnel utilisable et démontrable

---

## 🎯 Vision

Créer une interface **AI-native conversationnelle** où l'utilisateur dialogue en français avec son serveur mail via un LLM local.

**Différence clé avec interfaces classiques**:
- ❌ Pas de boutons "Nouveau message", "Répondre", "Supprimer"
- ✅ Chat: "Envoie un email à Marie pour lui dire bonjour"
- ✅ Natural language: "Liste mes emails de cette semaine"
- ✅ Context aware: "Réponds-lui que je suis d'accord"

---

## 🏗️ Architecture

```
┌─────────────────────────────────────┐
│         Frontend (Browser)          │
│  ┌─────────────────────────────┐   │
│  │   Chat Interface             │   │
│  │   - Input conversationnel    │   │
│  │   - Messages historique      │   │
│  │   - Streaming réponses LLM   │   │
│  └─────────────────────────────┘   │
│               ↕                     │
│          WebSocket                  │
└─────────────────────────────────────┘
               ↕
┌─────────────────────────────────────┐
│      ai-runtime (Backend)           │
│  - Port 8888                        │
│  - Streaming responses              │
│  - MCP orchestration                │
└─────────────────────────────────────┘
               ↕
┌─────────────────────────────────────┐
│    mcp-mail-server + mail-rs        │
│  - send_email, list_emails, etc     │
└─────────────────────────────────────┘
```

---

## 🛠️ Stack Technique

### Option A: React + Vite (Recommandé)
**Avantages**:
- ✅ Écosystème mature
- ✅ Vite ultra-rapide
- ✅ TypeScript natif
- ✅ Hot reload excellent
- ✅ Composants réutilisables

**Stack complète**:
```
- React 18 + TypeScript
- Vite (build tool)
- Tailwind CSS (styling)
- WebSocket API native
- React Router (navigation simple)
- Zustand/Context (state minimal)
```

### Option B: Svelte + SvelteKit
**Avantages**:
- ✅ Moins de boilerplate
- ✅ Performances excellentes
- ✅ Built-in animations
- ✅ Moins de code

**Inconvénient**:
- ⚠️ Écosystème plus petit

### Option C: Vue 3 + Vite
**Avantages**:
- ✅ Template syntax simple
- ✅ Composition API moderne
- ✅ Bonne DX

---

## 📐 Design & UX

### Layout Principal

```
┌────────────────────────────────────────┐
│  GK Mail - AI Assistant       [User]   │ <- Header
├────────────────────────────────────────┤
│                                        │
│  💬 Bonjour ! Je peux vous aider      │
│     avec vos emails.                   │
│                                        │
│  👤 Liste mes emails non lus           │
│                                        │
│  💬 Vous avez 3 emails non lus:        │
│     1. Marie - Réunion demain          │
│     2. Jean - Question urgente         │
│     3. Newsletter - Offre spéciale     │
│                                        │
│  👤 Réponds à Marie que je confirme    │
│                                        │
│  💬 ✅ Email envoyé à Marie !          │
│                                        │
├────────────────────────────────────────┤
│  [Votre message...]            [Envoi] │ <- Input
└────────────────────────────────────────┘
```

### Couleurs (Dark Mode par défaut)
```
- Background: #0f172a (slate-900)
- Surface: #1e293b (slate-800)
- Primary: #3b82f6 (blue-500)
- Text: #f1f5f9 (slate-100)
- Assistant: #8b5cf6 (purple-500)
- User: #3b82f6 (blue-500)
```

---

## 🚀 Roadmap Web UI

### Sprint 1: Foundation (3-4 jours)

**Objectif**: Interface chat basique fonctionnelle

- [ ] Setup projet Vite + React + TypeScript
- [ ] Configuration Tailwind CSS
- [ ] Layout de base (header + chat + input)
- [ ] Composant Message (user vs assistant)
- [ ] Composant ChatInput
- [ ] État local (messages array)

**Critère de succès**: Interface statique qui affiche des messages

### Sprint 2: WebSocket Integration (2-3 jours)

**Objectif**: Communication temps réel avec ai-runtime

- [ ] Connexion WebSocket à ai-runtime:8888
- [ ] Envoi de messages utilisateur
- [ ] Réception réponses AI (streaming)
- [ ] Gestion états connexion (connecting, connected, error)
- [ ] Affichage streaming token par token
- [ ] Auto-reconnexion

**Critère de succès**: Pouvoir envoyer "Liste mes emails" et recevoir la réponse

### Sprint 3: Email Visualization (2-3 jours)

**Objectif**: Affichage riche des emails dans le chat

- [ ] Parser réponses MCP (list_emails, read_email)
- [ ] Composant EmailCard (from, subject, date)
- [ ] Composant EmailList
- [ ] Actions inline (ouvrir, répondre)
- [ ] Loading states & skeletons

**Critère de succès**: Emails affichés joliment dans le chat

### Sprint 4: Polish & Features (2-3 jours)

**Objectif**: UX professionnelle

- [ ] Markdown rendering (réponses AI)
- [ ] Code syntax highlighting
- [ ] Scroll auto vers bas
- [ ] Timestamp messages
- [ ] Copier message
- [ ] Clear conversation
- [ ] Toast notifications
- [ ] Responsive mobile

**Critère de succès**: Expérience fluide et agréable

### Sprint 5: Auth & Deploy (1-2 jours)

**Objectif**: Sécurité basique et déploiement

- [ ] Auth simple (hardcoded users ou env)
- [ ] Page login basique
- [ ] JWT token storage
- [ ] Logout
- [ ] Build production
- [ ] Docker image web-ui
- [ ] Docker compose complet

**Critère de succès**: MVP déployable et sécurisé

---

## 📁 Structure Projet

```
web-ui/
├── public/
│   └── favicon.ico
├── src/
│   ├── components/
│   │   ├── Chat/
│   │   │   ├── Chat.tsx
│   │   │   ├── Message.tsx
│   │   │   ├── ChatInput.tsx
│   │   │   └── EmailCard.tsx
│   │   ├── Layout/
│   │   │   ├── Header.tsx
│   │   │   └── Sidebar.tsx (future)
│   │   └── ui/
│   │       ├── Button.tsx
│   │       ├── Input.tsx
│   │       └── Spinner.tsx
│   ├── hooks/
│   │   ├── useWebSocket.ts
│   │   └── useChat.ts
│   ├── lib/
│   │   ├── websocket.ts
│   │   └── api.ts
│   ├── types/
│   │   └── index.ts
│   ├── App.tsx
│   ├── main.tsx
│   └── index.css
├── package.json
├── tsconfig.json
├── vite.config.ts
├── tailwind.config.js
└── README.md
```

---

## 🔌 API ai-runtime

### WebSocket Endpoint

**Connection**: `ws://localhost:8888/ws`

**Message Format (Client → Server)**:
```json
{
  "type": "chat",
  "message": "Liste mes emails pour john@example.com"
}
```

**Response Format (Server → Client)**:
```json
{
  "type": "chunk",
  "content": "Voici vos emails..."
}

{
  "type": "tool_call",
  "tool": "list_emails",
  "arguments": { "email": "john@example.com" }
}

{
  "type": "tool_result",
  "tool": "list_emails",
  "result": {
    "count": 3,
    "emails": [...]
  }
}

{
  "type": "done",
  "content": "✅ Voici vos 3 emails"
}
```

⚠️ **Note**: Il faudra ajouter le support WebSocket dans ai-runtime (actuellement HTTP only)

---

## 🎬 Prochaines Actions Immédiates

### 1. Choisir la stack (Décision à prendre)
- [ ] React + Vite (Recommandé)
- [ ] Svelte + SvelteKit
- [ ] Vue 3 + Vite

### 2. Créer le projet
```bash
cd /home/kdelfour/Workspace/Personnel/Prototype/gk
npm create vite@latest web-ui -- --template react-ts
cd web-ui
npm install
npm install -D tailwindcss postcss autoprefixer
npx tailwindcss init -p
```

### 3. Premier composant Chat
- Layout de base
- Messages statiques
- Input fonctionnel

### 4. Ajouter WebSocket à ai-runtime
- Endpoint `/ws`
- Streaming responses
- MCP tool calls forwarding

---

## 🎯 Définition of Done (MVP)

Un utilisateur peut :
- ✅ Ouvrir l'interface dans son navigateur
- ✅ Voir un chat vide avec input
- ✅ Taper "Liste mes emails" et ENTER
- ✅ Voir la réponse de l'IA streamer en temps réel
- ✅ Voir les emails affichés joliment
- ✅ Taper "Envoie un email à X" et ça marche
- ✅ Naviguer sur mobile (responsive)

---

## 💡 Recommandation

**Démarrer avec React + Vite + Tailwind**

**Raisons**:
1. Écosystème mature et bien documenté
2. TypeScript excellent
3. Hot reload ultra-rapide
4. Facile de recruter/partager le code
5. Composants réutilisables pour la suite

**Prêt à coder ? 🚀**
