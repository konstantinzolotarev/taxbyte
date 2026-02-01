// Theme management for TaxByte

/**
 * Toggle between light and dark theme
 */
function toggleTheme() {
  const isDark = document.documentElement.classList.toggle('dark');
  const theme = isDark ? 'dark' : 'light';
  localStorage.setItem('theme', theme);
}

/**
 * Initialize theme on page load
 */
(function initTheme() {
  const theme = localStorage.getItem('theme');

  if (theme === 'dark') {
    document.documentElement.classList.add('dark');
  } else if (!theme && window.matchMedia('(prefers-color-scheme: dark)').matches) {
    // No saved preference, use system preference
    document.documentElement.classList.add('dark');
    localStorage.setItem('theme', 'dark');
  }
})();
