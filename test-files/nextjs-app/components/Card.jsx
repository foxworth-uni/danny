// COMPLETELY UNUSED COMPONENT - Never imported!
import React from 'react';

export default function Card({ title, content }) {
    return (
        <div className="card">
            <h3>{title}</h3>
            <p>{content}</p>
        </div>
    );
}

// Also unused
export function CardList({ cards }) {
    return (
        <div className="card-list">
            {cards.map(card => (
                <Card key={card.id} {...card} />
            ))}
        </div>
    );
}
