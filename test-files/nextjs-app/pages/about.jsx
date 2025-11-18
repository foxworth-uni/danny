// Next.js About Page
import React from 'react';
import Header from '../components/Header';

export default function About() {
    return (
        <div>
            <Header />
            <main>
                <h1>About Us</h1>
                <p>This is the about page.</p>
            </main>
        </div>
    );
}

// Dead code after return in a helper
function helperWithDeadCode() {
    return true;
    
    // Unreachable
    const deadCode = "unreachable";
    console.log(deadCode);
}
