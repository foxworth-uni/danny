// Used component - imported in pages
import React from 'react';

export default function Header() {
    return (
        <header className="header">
            <nav>
                <a href="/">Home</a>
                <a href="/about">About</a>
                <a href="/contact">Contact</a>
            </nav>
        </header>
    );
}

// Unused helper function in this component file
function unusedHelperFunction() {
    return "This is never called";
}

const unusedConstant = "Also never used";
