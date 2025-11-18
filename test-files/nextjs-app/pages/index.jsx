// Next.js Home Page - This uses some components
import React from 'react';
import Header from '../components/Header';
import Button from '../components/Button';

export default function Home() {
    const handleClick = () => {
        console.log('Button clicked!');
    };
    
    // Unused variable in this page
    const unusedPageVariable = "This is declared but never used";
    
    return (
        <div>
            <Header />
            <main>
                <h1>Welcome to My Next.js App</h1>
                <Button onClick={handleClick} variant="primary">
                    Click Me
                </Button>
            </main>
        </div>
    );
}

// Next.js data fetching - this should be kept even if it looks "unused"
export async function getServerSideProps(context) {
    return {
        props: {
            data: 'server data'
        }
    };
}

// Unused helper function in the page
function unusedPageHelper() {
    return "Never called";
}
