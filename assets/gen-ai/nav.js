// Progressive enhancement for Previous and Next.
//
// Without this, the nav form is a plain POST that renders the whole page: it
// already works. This upgrades it to swap only the lesson region, which is
// what keeps the workbench beside it mounted with its files.
//
// The one thing it must not do is swap across a policy boundary. The overview
// has no workbench and a style-only CSP; a call step has the workbench and the
// widened `script-src`. A fragment swap cannot change the document's CSP meta,
// so a transition that changes `data-workbench` falls back to a full
// navigation, where the server sends the right policy. In practice that is
// only overview-to-first-call and back.

'use strict';

const LESSON = 'lab-lesson';

function currentWorkbenchState() {
  const inner = document.querySelector('#' + LESSON + ' .lab-lesson__inner');
  return inner ? inner.dataset.workbench : null;
}

async function swap(scenarioId, step, go) {
  const body = new URLSearchParams({ step: String(step), go });
  const res = await fetch('/gen-ai/' + encodeURIComponent(scenarioId) + '/lesson', {
    method: 'POST',
    headers: { 'content-type': 'application/x-www-form-urlencoded' },
    body,
  });
  if (!res.ok) throw new Error(String(res.status));
  return res.text();
}

function wantsWorkbench(fragmentHtml) {
  // Read the incoming step's intent without trusting it into the DOM first.
  const m = fragmentHtml.match(/data-workbench="(true|false)"/);
  return m ? m[1] : null;
}

function enhance() {
  const lesson = document.getElementById(LESSON);
  if (!lesson) return;

  lesson.addEventListener('submit', async (ev) => {
    const form = ev.target.closest('.lab-nav');
    if (!form) return;

    // The scenario id is the last path segment of the form action.
    const action = form.getAttribute('action') || '';
    const scenarioId = action.split('/').pop();
    const step = Number(form.querySelector('input[name=step]').value);

    // Which button was pressed. The submitter carries it; fall back to next.
    const go = (ev.submitter && ev.submitter.value) || 'next';

    ev.preventDefault();

    let fragment;
    try {
      fragment = await swap(scenarioId, step, go);
    } catch {
      // Anything unexpected: let the native submit render the full page.
      form.submit();
      return;
    }

    // If the destination's workbench state differs from here, the CSP differs
    // too, and a swap would leave the document under the wrong policy. Navigate
    // instead, so the server sends the correct one.
    if (wantsWorkbench(fragment) !== currentWorkbenchState()) {
      form.submit();
      return;
    }

    lesson.innerHTML = fragment;

    // Re-sync the persistent workbench to the new step. The swap replaces only
    // the lesson, so without this the workbench beside it still holds the
    // previous step's seed files, checks, and runnable flag — the config-step
    // Check would grade against the wrong answer key. The fragment carries the
    // new step's payload under `#lab-step-data`; hand it to the mounted
    // workbench, which re-seeds its files and checks without tearing down an
    // already-booted runtime.
    const stepData = lesson.querySelector('#lab-step-data');
    if (stepData && window.__labWorkbench && typeof window.__labWorkbench.resync === 'function') {
      try {
        window.__labWorkbench.resync(JSON.parse(stepData.textContent));
      } catch (err) {
        // A malformed payload should fall back to a full navigation rather
        // than leave a mismatched workbench in place.
        // eslint-disable-next-line no-console
        console.error('[nav] workbench resync failed, navigating', err);
        form.submit();
        return;
      }
    }

    // Focus the new region so a keyboard user is not stranded on a button
    // that was just replaced.
    lesson.setAttribute('tabindex', '-1');
    lesson.focus();
  });
}

if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', enhance);
} else {
  enhance();
}
