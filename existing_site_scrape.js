// To be used with "./src/bin/import_existing.rs".
//
// Run in browser devtools on: https://zdvartcc.org/roster

const tables = [...document.querySelectorAll("table")];
const data = tables.map((table) => {
  const tds = table.querySelectorAll("td");
  const ois = tds[0].innerText;
  const name = tds[2].innerText.split("\n")[0];
  let certs = [];
  for (const section of tds[3].querySelectorAll("div>div")) {
    const name = section.innerText;
    let level = [...section.classList]
      .find((cn) => cn.startsWith("cert-color-"))
      .substr(11);
    level = level.charAt(0).toUpperCase() + level.slice(1);
    certs.push({ name, level });
  }
  return {
    ois,
    name,
    certs,
  };
});
copy(JSON.stringify(data));
