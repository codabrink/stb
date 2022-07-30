document.addEventListener("keydown", e => {
  switch (e.key) {
    case "s":
      const input = document.querySelector('input[name=q]')
      if (document.activeElement === input) return
      input.focus()
      e.preventDefault()
      break
    case "ArrowRight":
      let nextButton = document.querySelector("a#next")
      if (nextButton) nextButton.click()
      break
    case "ArrowLeft":
      let prevButton = document.querySelector("a#prev")
      if (prevButton) prevButton.click()
      break
    default:
  }
})


