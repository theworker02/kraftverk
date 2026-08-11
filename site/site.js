/**
 * Kraftverk site interactions — scroll reveal, nav, cascade bars, soft presence.
 * Respects prefers-reduced-motion.
 */
(function () {
  const reduce =
    window.matchMedia &&
    window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  /* Sticky nav state */
  const top = document.querySelector(".top");
  if (top) {
    const onScroll = () => {
      top.classList.toggle("is-scrolled", window.scrollY > 8);
    };
    onScroll();
    window.addEventListener("scroll", onScroll, { passive: true });

    const toggle = document.querySelector(".nav-toggle");
    if (toggle) {
      toggle.addEventListener("click", () => {
        const open = top.classList.toggle("is-open");
        toggle.setAttribute("aria-expanded", open ? "true" : "false");
      });
    }
  }

  /* Scroll reveal */
  const reveals = document.querySelectorAll(".reveal, .cascade");
  if (!reduce && "IntersectionObserver" in window) {
    const io = new IntersectionObserver(
      (entries) => {
        entries.forEach((e) => {
          if (e.isIntersecting) {
            e.target.classList.add("is-visible");
            io.unobserve(e.target);
          }
        });
      },
      { threshold: 0.15, rootMargin: "0px 0px -40px 0px" }
    );
    reveals.forEach((el) => io.observe(el));
  } else {
    reveals.forEach((el) => el.classList.add("is-visible"));
  }

  /* Animate architecture flow SVG when visible */
  const flow = document.querySelector(".flow-animate");
  if (flow && !reduce && "IntersectionObserver" in window) {
    const fio = new IntersectionObserver(
      (entries) => {
        entries.forEach((e) => {
          if (e.isIntersecting) {
            e.target.classList.add("is-animating");
            fio.unobserve(e.target);
          }
        });
      },
      { threshold: 0.25 }
    );
    fio.observe(flow);
  } else if (flow) {
    flow.classList.add("is-animating");
  }

  /* Download OS panels */
  const tabs = document.querySelectorAll(".os-tabs button");
  if (tabs.length) {
    tabs.forEach((btn) => {
      btn.addEventListener("click", () => {
        const id = btn.getAttribute("data-panel");
        tabs.forEach((b) => b.setAttribute("aria-selected", "false"));
        btn.setAttribute("aria-selected", "true");
        document.querySelectorAll(".os-panel").forEach((p) => {
          p.hidden = p.id !== id;
        });
      });
    });
  }

  /* Year in footer if present */
  document.querySelectorAll("[data-year]").forEach((el) => {
    el.textContent = String(new Date().getFullYear());
  });
})();
