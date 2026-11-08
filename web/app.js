document.addEventListener('DOMContentLoaded', () => {
    // Server Validator
    const statusInd = document.getElementById('status-indicator');
    const statusText = document.getElementById('status-text');
    
    async function checkHealth() {
        try {
            const resp = await fetch('/api/health');
            if (resp.ok) {
                statusInd.className = 'online';
                statusText.textContent = 'Online';
            } else throw new Error();
        } catch {
            statusInd.className = 'offline';
            statusText.textContent = 'Offline';
        }
    }
    setInterval(checkHealth, 3000);
    checkHealth();

    // Custom Select
    const select = document.getElementById('format-select');
    const header = select.querySelector('.select-header');
    const options = select.querySelector('.select-options');
    header.addEventListener('click', () => options.hidden = !options.hidden);
    options.addEventListener('click', (e) => {
        if(e.target.tagName === 'LI') {
            header.textContent = e.target.textContent;
            select.dataset.value = e.target.dataset.value;
            options.hidden = true;
        }
    });

    // Form logic (simplified for this turn)
    document.getElementById('yoink-form').addEventListener('submit', async (e) => {
        e.preventDefault();
        const url = document.getElementById('url-input').value;
        const format = select.dataset.value || 'best';
        const playlist = document.getElementById('playlist-check').checked;

        await fetch('/api/jobs', {
            method: 'POST',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify({url, format, playlist})
        });
        // (UI update logic omitted for brevity)
    });
});
