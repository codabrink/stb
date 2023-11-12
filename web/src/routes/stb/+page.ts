import type { PageLoad } from './$types';

export const load: PageLoad = ( { }) => {
  let foo =  fetch("http://localhost:8080/q?q=hello")
  .then(response => response.json())
  .then(data => {
    debugger
  }).catch(error => {
    debugger
    console.log(error)
  });


  return {

  }
}
