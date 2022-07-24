document.addEventListener("keydown", e => {
  switch (e.key) {
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


