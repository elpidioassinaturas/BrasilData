// A busca e os filtros do catálogo do BrasilData (scripts/site.py). Sem dependências.
(function () {
  "use strict";
  var semAcento = function (t) {
    return t.toLowerCase().normalize("NFD").replace(/[̀-ͯ]/g, "");
  };
  var busca = document.getElementById("busca");
  var fTopico = document.getElementById("f-topico");
  var fOrgao = document.getElementById("f-orgao");
  var fWeb = document.getElementById("f-web");
  var fMapa = document.getElementById("f-mapa");
  var resultado = document.getElementById("resultado");
  var vazio = document.getElementById("vazio");
  var cartoes = Array.prototype.slice.call(document.querySelectorAll("#catalogo .camada"));
  var textos = cartoes.map(function (c) { return semAcento(c.textContent); });
  var subtopicos = Array.prototype.slice.call(document.querySelectorAll("#catalogo .subtopico"));
  var topicos = Array.prototype.slice.call(document.querySelectorAll("#catalogo .topico"));
  var total = cartoes.filter(function (c) { return !c.classList.contains("eco"); }).length;

  function filtrar() {
    var termos = semAcento(busca.value.trim()).split(/\s+/).filter(Boolean);
    var t = fTopico.value, o = fOrgao.value, w = fWeb.value, m = fMapa.checked;
    var mostradas = 0;
    cartoes.forEach(function (c, i) {
      var ok = (!t || c.dataset.topico === t) &&
        (!o || c.dataset.orgao === o) &&
        (!w || c.dataset.web === w) &&
        (!m || c.dataset.mapa === "1") &&
        termos.every(function (x) { return textos[i].indexOf(x) !== -1; });
      c.hidden = !ok;
      if (ok && !c.classList.contains("eco")) mostradas++;
    });
    // Cada tópico e subtópico conta as camadas que passam no filtro (as repetidas também, no subtópico).
    subtopicos.concat(topicos).forEach(function (s) {
      var seletor = s.classList.contains("topico") ? ".camada:not(.eco):not([hidden])" : ".camada:not([hidden])";
      var n = s.querySelectorAll(seletor).length;
      s.hidden = !s.querySelector(".camada:not([hidden])");
      var conta = s.querySelector("h3 .conta, h4 .conta");
      if (conta) conta.textContent = n;
    });
    var filtrando = termos.length || t || o || w || m;
    resultado.textContent = filtrando
      ? mostradas + (mostradas === 1 ? " camada de " : " camadas de ") + total
      : total + " camadas. Abra uma para ver a ficha.";
    vazio.hidden = mostradas > 0;
    try {
      var p = new URLSearchParams();
      if (busca.value.trim()) p.set("busca", busca.value.trim());
      if (t) p.set("topico", t);
      if (o) p.set("orgao", o);
      if (w) p.set("web", w);
      if (m) p.set("mapa", "1");
      var q = p.toString();
      history.replaceState(null, "", (q ? "?" + q : location.pathname) + location.hash);
    } catch (e) { /* sem history: só não guarda os filtros no endereço */ }
  }

  function abrirDoEndereco() {
    var id = decodeURIComponent(location.hash.slice(1));
    if (!id) return;
    var alvo = document.getElementById(id);
    if (alvo && alvo.tagName === "DETAILS") {
      if (alvo.hidden) { limpar(); }
      alvo.open = true;
      alvo.scrollIntoView();
    }
  }

  function limpar() {
    busca.value = ""; fTopico.value = ""; fOrgao.value = ""; fWeb.value = ""; fMapa.checked = false;
    filtrar();
  }

  try {
    var p = new URLSearchParams(location.search);
    busca.value = p.get("busca") || "";
    fTopico.value = p.get("topico") || "";
    fOrgao.value = p.get("orgao") || "";
    fWeb.value = p.get("web") || "";
    fMapa.checked = p.get("mapa") === "1";
  } catch (e) { /* navegador antigo: começa sem filtro */ }

  var espera;
  busca.addEventListener("input", function () { clearTimeout(espera); espera = setTimeout(filtrar, 120); });
  [fTopico, fOrgao, fWeb, fMapa].forEach(function (el) { el.addEventListener("change", filtrar); });
  window.addEventListener("hashchange", abrirDoEndereco);
  // Um link "c-<id>" de uma camada repetida abre a ficha principal.
  document.addEventListener("click", function (ev) {
    var a = ev.target.closest && ev.target.closest('a[href^="#c-"]');
    if (a && a.getAttribute("href") === location.hash) abrirDoEndereco();
  });
  filtrar();
  abrirDoEndereco();
})();
