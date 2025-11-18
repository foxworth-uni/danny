// React JSX example with dead code
import React from 'react';

// Unused React component
function UnusedComponent() {
    return <div>I'm never rendered</div>;
}

// Used React component
function UsedComponent({ title }) {
    return <h1>{title}</h1>;
}

// Unused hook-like function
function useUnusedHook() {
    return { data: null };
}

// Used hook-like function
function useUsedHook() {
    return { count: 0 };
}

// Main App component that uses some things
function App() {
    const { count } = useUsedHook();
    
    return (
        <div>
            <UsedComponent title="Hello World" />
            <p>Count: {count}</p>
        </div>
    );
}

export default App;
