/**
 * yoink - minimal, smooth, and interactive video downloader UI
 */
(function () {
  'use strict';

  // DOM Elements
  const form = document.getElementById('yoink-form');
  const urlInput = document.getElementById('url-input');
  const formatChips = document.getElementById('format-chips');
  const playlistToggle = document.getElementById('playlist-toggle');
  const formError = document.getElementById('form-error');
  const queueList = document.getElementById('queue-list');
  const emptyState = document.getElementById('empty-state');
  const jobCount = document.getElementById('job-count');
  const backendStatus = document.getElementById('backendStatus');
  const statusText = document.getElementById('statusText');
  const toastEl = document.getElementById('toast');

  let selectedFormat = 'best';
  let isPlaylist = false;
  let activeJobsCount = 0;
  const activeSseMap = new Map();

  // Toast Helper
  let toastTimeout;
  function showToast(msg) {
    clearTimeout(toastTimeout);
    toastEl.textContent = `// ${msg}`;
    toastEl.classList.add('show');
    toastTimeout = setTimeout(() => {
      toastEl.classList.remove('show');
    }, 2400);
  }

  // Update item counter
  function updateCount(delta) {
    activeJobsCount = Math.max(0, activeJobsCount + delta);
    if (jobCount) {
      jobCount.textContent = `${activeJobsCount} item${activeJobsCount === 1 ? '' : 's'}`;
    }
    if (emptyState) {
      emptyState.style.display = activeJobsCount === 0 ? 'block' : 'none';
    }
  }

  // Format selection chips
  if (formatChips) {
    formatChips.addEventListener('click', (e) => {
      const btn = e.target.closest('.chip');
      if (!btn) return;
      formatChips.querySelectorAll('.chip').forEach(c => c.classList.remove('active'));
      btn.classList.add('active');
      selectedFormat = btn.dataset.value;
    });
  }

  // Playlist toggle button
  if (playlistToggle) {
    playlistToggle.addEventListener('click', () => {
      isPlaylist = !isPlaylist;
      playlistToggle.classList.toggle('active', isPlaylist);
    });
  }

  // Health / Backend status polling
  async function checkBackendStatus() {
    if (!backendStatus || !statusText) return;
    try {
      const res = await fetch('/api/health');
      if (res.ok) {
        backendStatus.className = 'backend-status online';
        statusText.textContent = 'online';
      } else {
        throw new Error('Non-200');
      }
    } catch {
      backendStatus.className = 'backend-status offline';
      statusText.textContent = 'offline';
    }
  }

  if (backendStatus) {
    backendStatus.addEventListener('click', () => {
      statusText.textContent = 'checking…';
      checkBackendStatus();
    });
  }

  // Form submission
  form.addEventListener('submit', async (e) => {
    e.preventDefault();
    formError.hidden = true;
    formError.textContent = '';

    const url = urlInput.value.trim();
    if (!url) return;

    try {
      const resp = await fetch('/api/jobs', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          url,
          format: selectedFormat,
          playlist: isPlaylist,
        }),
      });

      if (!resp.ok) {
        const errJson = await resp.json().catch(() => ({}));
        throw new Error(errJson.error || `Server responded with ${resp.status}`);
      }

      const { job_id } = await resp.json();
      urlInput.value = '';
      showToast('download queued');
      addJobCard(job_id, url, selectedFormat);
    } catch (err) {
      formError.textContent = `✗ ${err.message}`;
      formError.hidden = false;
    }
  });

  function addJobCard(jobId, url, format) {
    updateCount(1);

    const card = document.createElement('div');
    card.className = 'job-card';
    card.id = `job-${jobId}`;

    card.innerHTML = `
      <div class="job-main-row">
        <span class="job-title" id="title-${jobId}" title="${escapeHtml(url)}">${escapeHtml(url)}</span>
        <div class="job-actions" id="actions-${jobId}">
          <button type="button" class="action-btn action-delete" id="cancel-${jobId}">cancel</button>
        </div>
      </div>
      <div class="progress-wrap">
        <div class="progress-bar" id="bar-${jobId}"></div>
      </div>
      <div class="job-meta-row">
        <span class="job-status-text" id="status-text-${jobId}">connecting to stream…</span>
        <span class="job-status-badge" id="badge-${jobId}">${escapeHtml(format)}</span>
      </div>
    `;

    queueList.prepend(card);

    const cancelBtn = card.querySelector(`#cancel-${jobId}`);
    cancelBtn.addEventListener('click', async () => {
      await fetch(`/api/jobs/${jobId}`, { method: 'DELETE' });
      closeSse(jobId);
      card.remove();
      updateCount(-1);
      showToast('download cancelled');
    });

    connectSSE(jobId, card);
  }

  function connectSSE(jobId, card) {
    const sse = new EventSource(`/api/jobs/${jobId}/events`);
    activeSseMap.set(jobId, sse);

    const bar = card.querySelector(`#bar-${jobId}`);
    const statusTextEl = card.querySelector(`#status-text-${jobId}`);
    const badge = card.querySelector(`#badge-${jobId}`);
    const actions = card.querySelector(`#actions-${jobId}`);
    const titleEl = card.querySelector(`#title-${jobId}`);

    sse.onmessage = (e) => {
      try {
        const event = JSON.parse(e.data);
        handleProgressEvent(event, bar, statusTextEl, badge, actions, titleEl, jobId);
      } catch (err) {
        console.error('Failed to parse SSE event', err);
      }
    };

    sse.onerror = () => {
      fetch(`/api/jobs/${jobId}`)
        .then(r => r.json())
        .then(job => {
          if (job.status === 'completed' || job.status === 'failed') {
            closeSse(jobId);
          }
        })
        .catch(() => {});
    };
  }

  function closeSse(jobId) {
    if (activeSseMap.has(jobId)) {
      activeSseMap.get(jobId).close();
      activeSseMap.delete(jobId);
    }
  }

  function handleProgressEvent(event, bar, statusTextEl, badge, actions, titleEl, jobId) {
    const type = event.type;
    const data = event.data;

    if (type === 'Probing') {
      badge.textContent = 'probing';
      statusTextEl.textContent = data.status || 'probing metadata…';
      bar.style.width = '10%';
    } else if (type === 'Downloading') {
      badge.textContent = 'downloading';
      const downloaded = data.downloaded_bytes || 0;
      const total = data.total_bytes || 0;

      let pct = 0;
      if (total > 0) {
        pct = Math.min(100, Math.round((downloaded / total) * 100));
      }
      bar.style.width = `${pct}%`;

      const speed = data.speed_bps ? `${formatBytes(data.speed_bps)}/s` : '';
      const eta = data.eta_seconds ? `${formatDuration(data.eta_seconds)} left` : '';
      const part = data.total_parts > 1 ? `[${data.part + 1}/${data.total_parts}] ` : '';

      statusTextEl.textContent = `${part}${pct}% · ${formatBytes(downloaded)} / ${formatBytes(total)} ${speed} ${eta}`.trim();
    } else if (type === 'Processing') {
      badge.textContent = 'processing';
      bar.style.width = '100%';
      statusTextEl.textContent = data.step || 'processing formats…';
    } else if (type === 'Finished') {
      badge.textContent = 'complete';
      badge.className = 'job-status-badge completed';
      bar.style.width = '100%';
      statusTextEl.textContent = '✓ download ready';

      actions.innerHTML = `
        <a href="/api/jobs/${jobId}/file" class="action-btn action-download" download>download</a>
        <button type="button" class="action-btn action-delete" id="del-${jobId}">dismiss</button>
      `;

      actions.querySelector(`#del-${jobId}`).addEventListener('click', async () => {
        await fetch(`/api/jobs/${jobId}`, { method: 'DELETE' });
        actions.closest('.job-card').remove();
        updateCount(-1);
      });

      closeSse(jobId);
      showToast('yoink finished');
    } else if (type === 'Error') {
      badge.textContent = 'failed';
      badge.className = 'job-status-badge failed';
      statusTextEl.textContent = `✗ ${data.message || 'download error'}`;
      closeSse(jobId);
      showToast('download failed');
    }
  }

  function formatBytes(bytes) {
    if (!bytes || bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  }

  function formatDuration(seconds) {
    if (!seconds || seconds <= 0) return '0:00';
    const s = Math.round(seconds);
    const m = Math.floor(s / 60);
    const sec = s % 60;
    return `${m}:${sec.toString().padStart(2, '0')}`;
  }

  function escapeHtml(str) {
    return String(str)
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#39;');
  }

  async function fetchAvailableFiles() {
    try {
      const res = await fetch('/api/files');
      if (!res.ok) return;
      const files = await res.json();
      if (Array.isArray(files)) {
        files.forEach(file => {
          if (!document.getElementById(`job-${file.id}`)) {
            renderCompletedCard(file);
          }
        });
      }
    } catch {}
  }

  function renderCompletedCard(file) {
    updateCount(1);
    const card = document.createElement('div');
    card.className = 'job-card';
    card.id = `job-${file.id}`;

    const title = file.filename || file.url;
    const sizeStr = file.filesize ? ` · ${formatBytes(file.filesize)}` : '';

    card.innerHTML = `
      <div class="job-main-row">
        <span class="job-title" id="title-${file.id}" title="${escapeHtml(title)}">${escapeHtml(title)}</span>
        <div class="job-actions" id="actions-${file.id}">
          <a href="/api/jobs/${file.id}/file" class="action-btn action-download" download>download</a>
          <button type="button" class="action-btn action-delete" id="del-${file.id}">dismiss</button>
        </div>
      </div>
      <div class="progress-wrap">
        <div class="progress-bar" style="width: 100%;"></div>
      </div>
      <div class="job-meta-row">
        <span class="job-status-text">✓ cached on server${sizeStr}</span>
        <span class="job-status-badge completed">${escapeHtml(file.format)}</span>
      </div>
    `;

    queueList.appendChild(card);

    card.querySelector(`#del-${file.id}`).addEventListener('click', async () => {
      await fetch(`/api/jobs/${file.id}`, { method: 'DELETE' });
      card.remove();
      updateCount(-1);
    });
  }

  // Init
  checkBackendStatus();
  fetchAvailableFiles();
  setInterval(checkBackendStatus, 15000);
})();
