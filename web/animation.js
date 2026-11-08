/**
 * Interactive constellation particles matching katdrop (https://kd.oiupoyt.space)
 */
(function () {
  'use strict';

  function initInteractiveParticles() {
    const canvas = document.getElementById('particles');
    if (!canvas) return;
    const ctx = canvas.getContext('2d');

    let width = (canvas.width = window.innerWidth);
    let height = (canvas.height = window.innerHeight);

    window.addEventListener('resize', () => {
      width = canvas.width = window.innerWidth;
      height = canvas.height = window.innerHeight;
    });

    const pointer = {
      x: null,
      y: null,
      radius: 150,
    };

    window.addEventListener('mousemove', (e) => {
      pointer.x = e.clientX;
      pointer.y = e.clientY;
    });

    window.addEventListener('mouseleave', () => {
      pointer.x = null;
      pointer.y = null;
    });

    window.addEventListener('touchmove', (e) => {
      if (e.touches.length > 0) {
        pointer.x = e.touches[0].clientX;
        pointer.y = e.touches[0].clientY;
      }
    }, { passive: true });

    window.addEventListener('touchend', () => {
      pointer.x = null;
      pointer.y = null;
    });

    window.addEventListener('click', (e) => {
      for (const p of particles) {
        const dx = p.x - e.clientX;
        const dy = p.y - e.clientY;
        const dist = Math.sqrt(dx * dx + dy * dy);
        if (dist < 220 && dist > 0) {
          const force = (220 - dist) / 220;
          p.disturbVx += (dx / dist) * force * 4;
          p.disturbVy += (dy / dist) * force * 4;
        }
      }
    });

    const particles = [];
    const count = Math.min(80, Math.max(35, Math.floor(width / 20)));

    for (let i = 0; i < count; i++) {
      const angle = Math.random() * Math.PI * 2;
      const speed = 0.35 + Math.random() * 0.45;
      particles.push({
        x: Math.random() * width,
        y: Math.random() * height,
        baseVx: Math.cos(angle) * speed,
        baseVy: Math.sin(angle) * speed,
        disturbVx: 0,
        disturbVy: 0,
        radius: Math.random() * 1.5 + 1,
        phase: Math.random() * Math.PI * 2,
        twinkleSpeed: 0.0018 + Math.random() * 0.0025,
      });
    }

    function animate(time) {
      ctx.clearRect(0, 0, width, height);

      // Connect particles to each other (constellation network)
      for (let i = 0; i < particles.length; i++) {
        for (let j = i + 1; j < particles.length; j++) {
          const dx = particles[i].x - particles[j].x;
          const dy = particles[i].y - particles[j].y;
          const distSq = dx * dx + dy * dy;
          if (distSq < 15625) { // 125px
            const dist = Math.sqrt(distSq);
            ctx.strokeStyle = `rgba(255, 255, 255, ${0.075 * (1 - dist / 125)})`;
            ctx.lineWidth = 0.8;
            ctx.beginPath();
            ctx.moveTo(particles[i].x, particles[i].y);
            ctx.lineTo(particles[j].x, particles[j].y);
            ctx.stroke();
          }
        }
      }

      // Update & draw each particle
      for (const p of particles) {
        const waveX = Math.sin(time * 0.0012 + p.phase) * 0.22;
        const waveY = Math.cos(time * 0.0015 + p.phase) * 0.22;

        p.x += p.baseVx + p.disturbVx + waveX;
        p.y += p.baseVy + p.disturbVy + waveY;

        p.disturbVx *= 0.94;
        p.disturbVy *= 0.94;

        const pad = 15;
        if (p.x < -pad) p.x = width + pad;
        if (p.x > width + pad) p.x = -pad;
        if (p.y < -pad) p.y = height + pad;
        if (p.y > height + pad) p.y = -pad;

        // Pointer connection
        if (pointer.x !== null && pointer.y !== null) {
          const mdx = pointer.x - p.x;
          const mdy = pointer.y - p.y;
          const mDistSq = mdx * mdx + mdy * mdy;

          if (mDistSq < pointer.radius * pointer.radius) {
            const mDist = Math.sqrt(mDistSq);
            ctx.strokeStyle = `rgba(255, 255, 255, ${0.2 * (1 - mDist / pointer.radius)})`;
            ctx.lineWidth = 0.9;
            ctx.beginPath();
            ctx.moveTo(p.x, p.y);
            ctx.lineTo(pointer.x, pointer.y);
            ctx.stroke();
          }
        }

        // Particle dot
        const alpha = 0.35 + Math.sin(time * p.twinkleSpeed + p.phase) * 0.25;
        ctx.fillStyle = `rgba(255, 255, 255, ${Math.max(0.15, alpha)})`;
        ctx.beginPath();
        ctx.arc(p.x, p.y, p.radius, 0, Math.PI * 2);
        ctx.fill();
      }

      requestAnimationFrame(animate);
    }

    requestAnimationFrame(animate);
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', initInteractiveParticles);
  } else {
    initInteractiveParticles();
  }
})();
