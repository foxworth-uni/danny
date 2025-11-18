// Next.js Contact Page
import React from 'react';
import Header from '../components/Header';
import Button from '../components/Button';

export default function Contact() {
    const handleSubmit = () => {
        alert('Form submitted!');
    };
    
    return (
        <div>
            <Header />
            <main>
                <h1>Contact Us</h1>
                <p>Get in touch with our team.</p>
                <Button onClick={handleSubmit} variant="primary">
                    Send Message
                </Button>
            </main>
        </div>
    );
}
