// Used component - imported in pages
import React from 'react';

interface ButtonProps {
    onClick?: () => void;
    children: React.ReactNode;
    variant?: 'primary' | 'secondary';
}

export default function Button({ onClick, children, variant = 'primary' }: ButtonProps) {
    return (
        <button 
            onClick={onClick}
            className={`btn btn-${variant}`}
        >
            {children}
        </button>
    );
}

// UNUSED: Alternative button component that's never exported or used
function AlternativeButton({ label }: { label: string }) {
    return <button>{label}</button>;
}

// UNUSED: Helper function
const formatButtonText = (text: string) => text.toUpperCase();
