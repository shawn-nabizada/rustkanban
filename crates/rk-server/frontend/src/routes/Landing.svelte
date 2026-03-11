<script>
  import { onMount } from 'svelte';
  import gsap from 'gsap';
  import { ScrollTrigger } from 'gsap/ScrollTrigger';

  gsap.registerPlugin(ScrollTrigger);

  const titleChars = [
    ...'Rust'.split('').map(c => ({ char: c, accent: false })),
    ...'Kanban'.split('').map(c => ({ char: c, accent: true })),
  ];

  onMount(() => {
    const tl = gsap.timeline({ defaults: { ease: 'power3.out' } });

    // Hero entrance: characters drop in with elastic overshoot
    tl.from('.title-char', {
      y: 50,
      opacity: 0,
      rotateX: -90,
      duration: 0.6,
      stagger: 0.04,
      ease: 'back.out(1.7)',
    })
    .from('.tagline', {
      y: 24,
      opacity: 0,
      duration: 0.7,
    }, '-=0.3')
    .from('.hero-glow', {
      scale: 0.5,
      opacity: 0,
      duration: 1.2,
      ease: 'power2.out',
    }, 0);

    // Feature rows slide up together
    gsap.from('.feature', {
      scrollTrigger: {
        trigger: '.features',
        start: 'top 82%',
      },
      y: 20,
      opacity: 0,
      duration: 0.5,
      stagger: 0.1,
      ease: 'power2.out',
    });

    // Sections slide up on scroll
    gsap.utils.toArray('.section').forEach(section => {
      gsap.from(section, {
        scrollTrigger: { trigger: section, start: 'top 85%' },
        y: 36,
        opacity: 0,
        duration: 0.6,
        ease: 'power2.out',
      });
    });

    // Terminal blocks stagger in
    gsap.utils.toArray('.terminal').forEach((el, i) => {
      gsap.from(el, {
        scrollTrigger: { trigger: el, start: 'top 88%' },
        x: i % 2 === 0 ? -30 : 30,
        opacity: 0,
        duration: 0.5,
        ease: 'power2.out',
      });
    });

    // Footer
    gsap.from('.footer', {
      scrollTrigger: { trigger: '.footer', start: 'top 95%' },
      opacity: 0,
      duration: 0.8,
    });

    return () => ScrollTrigger.getAll().forEach(t => t.kill());
  });
</script>

<div class="landing">
  <div class="hero">
    <div class="hero-glow"></div>
    <h1 class="title">
      {#each titleChars as ch}
        <span class="title-char" class:accent={ch.accent}>{ch.char}</span>
      {/each}
    </h1>
    <p class="tagline">A fast, keyboard-driven kanban board that lives in your terminal. Organize tasks, sync across devices, stay focused.</p>
  </div>

  <div class="features">
    <div class="feature">
      <span class="feature-icon">&#9000;</span>
      <div class="feature-text">
        <h3>Vim-style Navigation</h3>
        <p>Navigate columns with H/L, tasks with arrow keys. Everything is keyboard-first.</p>
      </div>
    </div>
    <div class="feature">
      <span class="feature-icon">&#8644;</span>
      <div class="feature-text">
        <h3>Cross-machine Sync</h3>
        <p>Log in with GitHub and sync your board across all your devices seamlessly.</p>
      </div>
    </div>
    <div class="feature">
      <span class="feature-icon">&#9998;</span>
      <div class="feature-text">
        <h3>Tags &amp; Search</h3>
        <p>Organize with tags, filter by tag, and search across all tasks instantly.</p>
      </div>
    </div>
  </div>

  <div class="section">
    <h2>Install</h2>
    <p>Quick install via script:</p>
    <div class="terminal">
      <div class="terminal-bar">
        <span class="dot red"></span><span class="dot yellow"></span><span class="dot green"></span>
        <span class="terminal-title">bash</span>
      </div>
      <div class="terminal-body">
        <span class="prompt">$</span>
        <code>curl -fsSL https://rustkanban.com/install.sh | sh</code>
        <span class="cursor"></span>
      </div>
    </div>
    <p>Or build from source with Cargo:</p>
    <div class="terminal">
      <div class="terminal-bar">
        <span class="dot red"></span><span class="dot yellow"></span><span class="dot green"></span>
        <span class="terminal-title">bash</span>
      </div>
      <div class="terminal-body">
        <span class="prompt">$</span>
        <code>cargo install --path crates/rk-client</code>
      </div>
    </div>
  </div>

  <div class="section">
    <h2>Sync Setup</h2>
    <p>One command to connect your board across machines.</p>
    <div class="terminal">
      <div class="terminal-bar">
        <span class="dot red"></span><span class="dot yellow"></span><span class="dot green"></span>
        <span class="terminal-title">bash</span>
      </div>
      <div class="terminal-body">
        <span class="prompt">$</span>
        <code>rk login</code>
      </div>
    </div>
    <p>Opens your browser for GitHub authentication. Your board syncs automatically when you open and close the app.</p>
  </div>

  <footer class="footer">RustKanban &mdash; a terminal kanban board</footer>
</div>

<style>
  .landing {
    max-width: 720px;
    margin: 0 auto;
    padding: 40px 20px;
    color: #c9d1d9;
    overflow: hidden;
  }

  /* ── Hero ── */
  .hero {
    text-align: center;
    padding: 56px 0 40px;
    position: relative;
  }
  .hero-glow {
    position: absolute;
    width: 420px; height: 420px;
    top: 50%; left: 50%;
    transform: translate(-50%, -55%);
    background: radial-gradient(circle, #64ffda10 0%, #bb86fc08 40%, transparent 70%);
    filter: blur(60px);
    pointer-events: none;
  }
  .title {
    font-size: 52px;
    font-weight: 800;
    margin: 0 0 16px;
    letter-spacing: -1.5px;
    perspective: 600px;
    position: relative;
    z-index: 1;
  }
  .title-char {
    display: inline-block;
    color: #e0e0e0;
    will-change: transform, opacity;
  }
  .title-char.accent { color: #64ffda; }
  .tagline {
    font-size: 17px;
    color: #8b949e;
    max-width: 500px;
    margin: 0 auto;
    line-height: 1.7;
    position: relative;
    z-index: 1;
  }

  /* ── Features ── */
  .features {
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin-bottom: 48px;
  }
  .feature {
    display: flex;
    align-items: center;
    gap: 20px;
    background: #16213e;
    border: 1px solid #1e2d4a;
    border-radius: 10px;
    padding: 20px 24px;
    transition: border-color 0.25s, transform 0.25s;
    will-change: transform;
  }
  .feature:hover {
    border-color: #64ffda25;
    transform: translateX(4px);
  }
  .feature-icon {
    font-size: 28px;
    color: #bb86fc;
    flex-shrink: 0;
    width: 48px;
    height: 48px;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .feature-text h3 {
    font-size: 16px;
    font-weight: 600;
    color: #e0e0e0;
    margin: 0 0 4px;
  }
  .feature-text p {
    font-size: 16px;
    color: #8b949e;
    margin: 0;
    line-height: 1.5;
  }

  /* ── Sections ── */
  .section {
    margin-bottom: 48px;
  }
  .section h2 {
    font-size: 20px;
    color: #e0e0e0;
    margin: 0 0 10px;
  }
  .section p {
    font-size: 16px;
    color: #8b949e;
    margin: 4px 0 14px;
    line-height: 1.5;
  }

  /* ── Terminal blocks ── */
  .terminal {
    background: #0a0f1a;
    border: 1px solid #1e2d4a;
    border-radius: 10px;
    overflow: hidden;
    margin-bottom: 18px;
    will-change: transform;
  }
  .terminal-bar {
    display: flex; align-items: center; gap: 6px;
    padding: 10px 14px;
    background: #141d30;
    border-bottom: 1px solid #1e2d4a;
  }
  .dot {
    width: 10px; height: 10px; border-radius: 50%;
  }
  .dot.red { background: #ff5f57; }
  .dot.yellow { background: #ffbd2e; }
  .dot.green { background: #28c840; }
  .terminal-title {
    margin-left: 8px;
    font-size: 16px;
    color: #484f58;
  }
  .terminal-body {
    padding: 16px 18px;
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .prompt {
    color: #6bcb77;
    font-weight: 700;
    user-select: none;
  }
  .terminal-body code {
    font-family: inherit;
    font-size: 16px;
    color: #64ffda;
  }
  .cursor {
    display: inline-block;
    width: 8px; height: 17px;
    background: #64ffda;
    border-radius: 1px;
    animation: blink 1.1s step-end infinite;
  }
  @keyframes blink {
    0%, 100% { opacity: 1; }
    50% { opacity: 0; }
  }

  /* ── Footer ── */
  .footer {
    text-align: center;
    color: #484f58;
    font-size: 16px;
    margin-top: 48px;
    padding-top: 24px;
    border-top: 1px solid #1e2d4a;
  }

  @media (max-width: 600px) {
    .title { font-size: 40px; }
    .hero-glow { width: 280px; height: 280px; }
  }
</style>
