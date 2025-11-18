# Next.js Test App for Danny

A real Next.js application created to test the **Danny** dead code detector.

## What's Inside

### Used Components ✅
- `Header.jsx` - Navigation header used on all pages
- `Button.tsx` - Reusable button component used on index and contact pages

### Unused Components ❌ (Dead Code!)
- `Footer.jsx` - Complete component, never imported
- `Card.jsx` - Two components (Card & CardList), never used
- `Sidebar.jsx` - Complete component, never imported

### Pages
- `index.jsx` - Home page (uses Header & Button)
- `about.jsx` - About page (uses Header)
- `contact.jsx` - Contact page (uses Header & Button)
- `_app.jsx` - Next.js app wrapper
- `_document.jsx` - Next.js document structure

## Running the App

### Install dependencies
```bash
npm install
```

### Run development server
```bash
npm run dev
```

Open [http://localhost:3000](http://localhost:3000)

### Build for production
```bash
npm run build
npm start
```

## Analyze Dead Code

### Run Danny on this app
```bash
npm run analyze
```

Or directly:
```bash
../../target/release/danny .
```

### Expected Results

Danny should detect:
- ❌ 3 unused components: Footer, Card, Sidebar
- ❌ ~12+ unused helper functions and variables
- ✅ Correctly identifies Header and Button as used
- ✅ Skips Next.js magic functions like `getServerSideProps`

## Project Structure

```
nextjs-app/
├── components/
│   ├── Header.jsx      ✅ USED
│   ├── Button.tsx      ✅ USED
│   ├── Footer.jsx      ❌ UNUSED
│   ├── Card.jsx        ❌ UNUSED
│   └── Sidebar.jsx     ❌ UNUSED
├── pages/
│   ├── _app.jsx
│   ├── _document.jsx
│   ├── index.jsx       (uses Header, Button)
│   ├── about.jsx       (uses Header)
│   └── contact.jsx     (uses Header, Button)
├── styles/
│   └── globals.css
├── public/
├── package.json
└── next.config.js
```

## What This Demonstrates

1. **Real Next.js setup** - Actual working app with proper structure
2. **Component usage patterns** - Some components used, some not
3. **Dead code detection** - Perfect test case for Danny
4. **TypeScript support** - Button.tsx shows TS component detection
5. **Internal dead code** - Helper functions inside components that are unused

## Clean Up Commands

After Danny identifies dead code, you can safely delete:
```bash
rm components/Footer.jsx
rm components/Card.jsx
rm components/Sidebar.jsx
```

This will reduce the codebase while keeping all functionality intact!
