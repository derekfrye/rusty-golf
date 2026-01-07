const storageKey = 'theme-preference'
const themes = ['classic', 'new', 'dark', 'modern']

const onClick = () => {
  // flip current value
  const currentIndex = themes.indexOf(theme.value)
  const nextIndex = currentIndex === -1 ? 0 : (currentIndex + 1) % themes.length
  theme.value = themes[nextIndex]

  setPreference()
}

const getColorPreference = () => {
  const stored = localStorage.getItem(storageKey)
  if (stored && themes.includes(stored))
    return stored
  else
    return window.matchMedia('(prefers-color-scheme: dark)').matches
      ? 'dark'
      : 'new'
}

const setPreference = () => {
  localStorage.setItem(storageKey, theme.value)
  reflectPreference()
}

const reflectPreference = () => {
  document.firstElementChild
    .setAttribute('data-theme', theme.value)

  document
    .querySelector('#theme-toggle')
    ?.setAttribute('aria-label', theme.value)

  const themeToggleText = document.querySelector('#theme-toggle-text')
  if (themeToggleText) {
    themeToggleText.textContent = theme.value
  }

  const themeStylesheet = document.querySelector('#theme-stylesheet')
  if (themeStylesheet?.dataset) {
    const dataKey = `theme${theme.value.charAt(0).toUpperCase()}${theme.value.slice(1)}`
    const nextHref = themeStylesheet.dataset[dataKey]
    if (nextHref && themeStylesheet.getAttribute('href') !== nextHref) {
      themeStylesheet.setAttribute('href', nextHref)
    }
  }
}

const theme = {
  value: getColorPreference(),
}

// set early so no page flashes / CSS is made aware
reflectPreference()

window.onload = () => {
  // set on load so screen readers can see latest value on the button
  reflectPreference()

  // now this script can find and listen for clicks on the control
  document
    .querySelector('#theme-toggle')
    .addEventListener('click', onClick)
}

// sync with system changes
window
  .matchMedia('(prefers-color-scheme: dark)')
  .addEventListener('change', ({matches:isDark}) => {
    theme.value = isDark ? 'dark' : 'new'
    setPreference()
  })
        
