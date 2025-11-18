// Page demonstrating various Knip issues including UNRESOLVED IMPORTS
import React from 'react';

// Example 1: UNRESOLVED IMPORT - Path alias that might not be configured
// import { theme } from '@/styles/theme';

// Example 2: UNRESOLVED IMPORT - Dynamic import with template string
// const dynamicModule = (name: string) => import(`./modules/${name}`);

// Example 3: UNRESOLVED IMPORT - Missing file extension in Next.js
// import { config } from '../config';  // if config.ts doesn't exist

// Example 4: UNRESOLVED IMPORT - Non-existent module
// import { nonExistent } from './this-does-not-exist';

// Example 5: Using a module that exists but demonstrates issues
import Button from '../components/Button';
import { formatCurrency } from '../utils/helpers';

// Example 6: Importing from node_modules that aren't installed
// import { v4 as uuidv4 } from 'uuid'; // uuid is not in package.json

interface DemoProps {
  title: string;
}

export default function DemoPage({ title }: DemoProps) {
  const price = 99.99;

  return (
    <div>
      <h1>{title || 'Demo Page'}</h1>
      <p>Price: {formatCurrency(price)}</p>
      <Button onClick={() => console.log('clicked')}>
        Click Here
      </Button>
    </div>
  );
}

// Unused getStaticProps - demonstrates dead code in Next.js context
export async function getStaticProps() {
  return {
    props: {
      title: 'Demo'
    }
  };
}
