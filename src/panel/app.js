const input = document.getElementById("diary");
const dateEl = document.getElementById("date");
const toastEl = document.getElementById("toast");

dateEl.textContent = new Date().toLocaleDateString("en-GB", {
  weekday: "long",
  day: "numeric",
  month: "long",
  year: "numeric",
});

function grow() {
  input.style.height = "auto";
  input.style.height =
    Math.min(input.scrollHeight, window.innerHeight * 0.5) + "px";
}

let toastTimer;
function toast(text, error) {
  toastEl.textContent = text;
  toastEl.classList.toggle("error", !!error);
  toastEl.classList.add("show");
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => toastEl.classList.remove("show"), 1800);
}

async function save() {
  const text = input.value.trim();
  if (!text) return;
  try {
    const r = await fetch("/add", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ module: "diary", text }),
    });
    if (!r.ok) throw new Error("server " + r.status);
    input.value = "";
    grow();
    toast("queued · " + new Date().toTimeString().slice(0, 5));
  } catch (e) {
    toast("not saved · " + e.message, true);
  }
}

input.addEventListener("input", grow);
input.addEventListener("keydown", (e) => {
  if (e.key === "Enter" && !e.shiftKey) {
    e.preventDefault();
    save();
  }
});
window.addEventListener("resize", grow);
grow();
