// UNUSED COMPONENT
import React from 'react';

export default function Sidebar() {
    const unusedLinks = ['Link 1', 'Link 2', 'Link 3'];
    
    return (
        <aside className="sidebar">
            <h2>Sidebar</h2>
        </aside>
    );
}

// Dead code: unused utility
function formatSidebarItem(item) {
    return item.toUpperCase();
}
