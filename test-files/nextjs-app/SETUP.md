# Setup Instructions

## Fix npm cache (if needed)

If you get npm permission errors, run:
```bash
sudo chown -R $(whoami) ~/.npm
```

Then install dependencies:
```bash
npm install
```

## Or use pnpm (recommended)

```bash
pnpm install
pnpm dev
```

## Or use yarn

```bash
yarn install
yarn dev
```

## Test Danny on This App

Once dependencies are installed (or even without them!), you can run Danny:

```bash
# From the nextjs-app directory
npm run analyze

# Or from the danny root
cd ../..
./target/release/danny test-files/nextjs-app/
```

## What You'll See

Danny will detect:
- 3 completely unused components (Footer, Card, Sidebar)
- Multiple unused helper functions
- Unused variables within components

All while the app runs perfectly fine with just Header and Button components!

## Running the App

```bash
npm run dev
# or
pnpm dev
# or  
yarn dev
```

Visit http://localhost:3000

You'll see a working Next.js app with:
- Home page
- About page
- Contact page
- Functional navigation
- Working buttons

...and NO errors from the "unused" components because they're truly not needed!
