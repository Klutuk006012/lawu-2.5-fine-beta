# Lawu-2.5-Fine Dashboard (Vue.js 3)

Dashboard Vue.js 3 untuk mesin pencari ebook Lawu-2.5-Fine.
Backend: Rust Axum + Tantivy + llama.cpp.

## Stack
- Vue.js 3 (Composition API)
- Vite 6 (build tool)
- Pinia (state management)
- Vue Router (multi-page)
- Tailwind CSS (styling)
- lucide-vue-next (icons)

## Development
```bash
bun install
bun run dev    # http://localhost:3001
```

## Production build
```bash
bun run build  # output: dist/
bun run preview # test production build
```

## Environment
Buat `.env.local`:
```
VITE_LAWU_BACKEND_URL=http://127.0.0.1:3001
VITE_LAWU_LLM_URL=http://127.0.0.1:8080
```

## Deploy ke srv-fine
```bash
# Clone repo
git clone https://github.com/Klutuk006012/lawu-dashboard.git
cd lawu-dashboard/vue-dashboard

# Install + build
bun install
bun run build

# Serve via nginx (recommended):
sudo cp -r dist/* /var/www/lawu-dashboard/
```

## Paten 2: Vue.js 3 (Vite + Pinia)
Dashboard ini WAJIB Vue.js 3, BUKAN Next.js/React. Sesuai Paten 2 proyek Lawu-2.5-Fine.
