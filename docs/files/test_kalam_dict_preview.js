// jsdom harness for docs/files/kalam_dictionary_popup_v3_preview.html.
// Run:  node docs/files/test_kalam_dict_preview.js
const fs = require('fs');
const { JSDOM } = require('jsdom');
const html = fs.readFileSync('/home/user/calibre-alt/docs/files/kalam_dictionary_popup_v3_preview.html', 'utf8');
// jsdom has no Range.getBoundingClientRect; the real WebKitGTK does. A
// small polyfill keeps the rect-based assertions meaningful.
const dom = new JSDOM(html, {
  runScripts: 'dangerously',
  pretendToBeVisual: true,
  beforeParse(window) {
    window.Range.prototype.getBoundingClientRect = function() {
      return { left: 12, top: 34, width: 56, height: 8, bottom: 42, right: 68 };
    };
  }
});
const { window } = dom;
const { document } = window;
let failures = 0;
function check(name, cond) { console.log((cond ? 'PASS' : 'FAIL') + '  ' + name); if (!cond) failures++; }

// Simulate a scrolled book: the popup must stay viewport-anchored.
try {
  Object.defineProperty(window, 'scrollY', { value: 300, configurable: true });
  Object.defineProperty(window, 'scrollX', { value: 150, configurable: true });
} catch(e) { console.log('note: cannot override scrollY in jsdom:', e.message); }

const payload = {
  word: 'set', pos: ['noun','verb','adjective'],
  senses: Array.from({length: 44}, (_, i) => ({number: i+1, pos: null, def: 'sense '+(i+1), example: null})),
  synonyms: ['a','b'], antonyms: ['c'], idioms: [{phrase:'Piece of cake', def:'Easy.'}],
  suggestions: [], saved: false, hint: 9
};
window.kalamShowDict(payload, JSON.stringify({x:0, y:40, w:40, h:24}));

const popup = document.getElementById('kalam-dict-popup');

// Dimensions
const cs = window.getComputedStyle(popup);
check('width 320px', cs.width === '320px');
check('position fixed', cs.position === 'fixed');
const bodyCs = window.getComputedStyle(popup.querySelector('.k-body'));
check('body max-height 300px', bodyCs.maxHeight === '300px');

// Viewport anchoring: scrollY is mocked at 300, so 300 would leak in if
// the popup were document-positioned. It must still be at 74 (view coords).
check('anchored in viewport despite scrollY=300', popup.style.top === '74px');
check('left ignores scrollX (clamped to 8)', popup.style.left === '8px');

// Header actions: save, find-in-chapter, copy (no close button)
check('3 header buttons (save/find/copy, no close)', popup.querySelectorAll('.k-save-btn').length === 3);
check('find button present', !!document.getElementById('kalam-dict-find'));
const copyBtn = document.getElementById('kalam-dict-copy');
copyBtn.click();
check('copy keeps popup open', popup.style.display === 'block');
check('copy feedback check', copyBtn.textContent === '\u2713');

// P5.5 hint: exactly one badge, on sense index 9 (the 10th item), labelled
// "likely here", and the sense keeps its number.
const items = popup.querySelectorAll('.k-def-item');
check('44 sense items', items.length === 44);
const badges = popup.querySelectorAll('.k-hint-badge');
check('exactly one hint badge', badges.length === 1);
check('badge text "likely here"', badges[0].textContent.trim() === 'likely here');
check('badge on sense index 9 (10th item)', items[9].querySelector('.k-hint-badge') !== null);
check('hinted sense keeps number', items[9].querySelector('.k-def-num').textContent === '10.');
check('no badge on first sense', items[0].querySelector('.k-hint-badge') === null);
const badgeCs = window.getComputedStyle(badges[0]);
check('badge has explicit color style', badgeCs.color !== '' && badgeCs.color !== 'rgba(0, 0, 0, 0)');

// Hint in the visible first-3 range (index 2 -> 3rd item)
window.kalamShowDict({word:'x', pos:[], senses:[{number:1,pos:null,def:'a',example:null},{number:2,pos:null,def:'b',example:null},{number:3,pos:null,def:'c',example:null}], synonyms:[], antonyms:[], idioms:[], suggestions:[], saved:false, hint:2}, null);
const items3 = popup.querySelectorAll('.k-def-item');
check('hint in shown range -> badge on 3rd', items3[2].querySelector('.k-hint-badge') !== null);
check('no badge on 1st/2nd when hint=2', items3[0].querySelector('.k-hint-badge') === null && items3[1].querySelector('.k-hint-badge') === null);

// No hint -> no badge at all
window.kalamShowDict({word:'x', pos:[], senses:[{number:1,pos:null,def:'d',example:null}], synonyms:[], antonyms:[], idioms:[], suggestions:[], saved:false}, null);
check('no hint key -> no badge', popup.querySelectorAll('.k-hint-badge').length === 0);
window.kalamShowDict({word:'x', pos:[], senses:[{number:1,pos:null,def:'d',example:null}], synonyms:[], antonyms:[], idioms:[], suggestions:[], saved:false, hint:null}, null);
check('hint null -> no badge', popup.querySelectorAll('.k-hint-badge').length === 0);

// Pronunciation slot (Phase 5.6)
const pron = () => document.getElementById('kalam-pronunciation');
window.kalamShowDict({word:'x', pos:[], senses:[{number:1,pos:null,def:'d',example:null}], synonyms:[], antonyms:[], idioms:[], suggestions:[], saved:false}, null);
check('no pronunciation key -> slot stays empty', pron().textContent === '' && pron().matches(':empty'));
window.kalamShowDict({word:'bank', pos:['noun'], senses:[{number:1,pos:'noun',def:'a financial institution',example:null}], synonyms:[], antonyms:[], idioms:[], suggestions:[], saved:false, pronunciation:'/\u02c8b\u00e6\u014bk/'}, null);
check('pronunciation payload -> slot shows /\u02c8b\u00e6\u014bk/', pron().textContent === '/\u02c8b\u00e6\u014bk/' && !pron().matches(':empty'));
window.kalamShowDict({word:'x', pos:[], senses:[{number:1,pos:null,def:'d',example:null}], synonyms:[], antonyms:[], idioms:[], suggestions:[], saved:false, pronunciation:null}, null);
check('pronunciation null -> slot cleared and hidden', pron().textContent === '' && pron().matches(':empty'));

// Bridge recording (overrides the preview's demo bridge)
let bridgeCalls = [];
window.kalamBridge = (m) => bridgeCalls.push(m);

// Chip lookup keeps position
window.kalamShowDict({word:'arrange', pos:['verb'], senses:[{number:1,pos:'verb',def:'to put in order',example:null}], synonyms:['put in order'], antonyms:[], idioms:[], suggestions:[], saved:false}, null);
const topBefore = popup.style.top, leftBefore = popup.style.left;
popup.querySelector('.k-chip-syn').click();
window.kalamShowDict({word:'arrange', pos:['verb'], senses:[{number:1,pos:'verb',def:'to put in order',example:null}], synonyms:[], antonyms:[], idioms:[], suggestions:[], saved:false}, null);
check('chip lookup keeps top', popup.style.top === topBefore);
check('chip lookup keeps left', popup.style.left === leftBefore);

// Click outside closes; inside keeps
document.body.click();
check('click outside closes', popup.style.display === 'none');
window.kalamShowDict({word:'x', pos:[], senses:[{number:1,pos:null,def:'d',example:null}], synonyms:[], antonyms:[], idioms:[], suggestions:[], saved:false}, null);
popup.querySelector('.k-def-item').click();
check('click inside keeps open', popup.style.display === 'block');

// Flip above when no room below
window.kalamShowDict({word:'y', pos:[], senses:[{number:1,pos:null,def:'d',example:null}], synonyms:[], antonyms:[], idioms:[], suggestions:[], saved:false},
  JSON.stringify({x:0, y:700, w:40, h:24}));
const topAbove = parseInt(popup.style.top);
check('flips above (470)', topAbove === 470);
check('clears selection', topAbove + 220 <= 700 - 10);

// Neither side fits -> cap body height
window.kalamShowDict({word:'z', pos:[], senses:[{number:1,pos:null,def:'d',example:null}], synonyms:[], antonyms:[], idioms:[], suggestions:[], saved:false},
  JSON.stringify({x:0, y:50, w:40, h:700}));
check('body capped to 60px', popup.querySelector('.k-body').style.maxHeight === '60px');
check('stays within viewport top', parseInt(popup.style.top) >= 8);

// New selection rect re-anchors
window.kalamShowDict({word:'w', pos:[], senses:[{number:1,pos:null,def:'d',example:null}], synonyms:[], antonyms:[], idioms:[], suggestions:[], saved:false},
  JSON.stringify({x:0, y:40, w:40, h:24}));
check('fresh lookup re-anchors to 74', popup.style.top === '74px');

// P5.5: sentence context — the selection alone must not be sent; the full
// surrounding sentence reaches the bridge.
function selectTextIn(el, needle) {
  const walker = document.createTreeWalker(el, window.NodeFilter.SHOW_TEXT);
  let node;
  while ((node = walker.nextNode())) {
    const idx = node.textContent.indexOf(needle);
    if (idx !== -1) {
      const range = document.createRange();
      range.setStart(node, idx);
      range.setEnd(node, idx + needle.length);
      const sel = window.getSelection();
      sel.removeAllRanges();
      sel.addRange(range);
      return true;
    }
  }
  return false;
}

const para = document.createElement('p');
para.textContent = 'The children played on the muddy bank of the stream. Then they went home.';
document.body.appendChild(para);
check('selection made', selectTextIn(para, 'bank'));
check('getContextSentence returns the full sentence',
  window.kalamGetContextSentence() === 'The children played on the muddy bank of the stream.');
window.kalamHandleDict();
const lookup = bridgeCalls.find(m => m.type === 'dict-lookup' && m.word === 'bank');
check('kalamHandleDict sends the full sentence as context',
  !!lookup && lookup.context === 'The children played on the muddy bank of the stream.');
check('kalamHandleDict keeps the selected word', !!lookup && lookup.word === 'bank');

// ---- Phase 6: tap-to-look-up ----
// wordFromCaret: expansion to word boundaries within a text node
const firstText = para.firstChild;
const bankIdx = firstText.data.indexOf('bank');
const playedIdx = firstText.data.indexOf('played');
check('wordFromCaret finds "bank" around its caret', (() => {
  const w = window.kalamWordFromCaret({node: firstText, offset: bankIdx + 2});
  return w && w.word === 'bank' && w.start === bankIdx && w.end === bankIdx + 4;
})());
check('wordFromCaret rect present', (() => {
  const w = window.kalamWordFromCaret({node: firstText, offset: bankIdx + 2});
  return !!(w && w.rect);
})());
check('wordFromCaret stops at punctuation', (() => {
  const w = window.kalamWordFromCaret({node: firstText, offset: playedIdx + 3}); // inside "played,"
  return w && w.word === 'played';
})());
check('wordFromCaret rejects empty/non-word', (() => {
  const p = document.createElement('p');
  p.textContent = '---';
  document.body.appendChild(p);
  return window.kalamWordFromCaret({node: p.firstChild, offset: 1}) === null;
})());
check('sentenceAroundText is case-insensitive', (() => {
  const p = document.createElement('p');
  p.textContent = 'The BANK of the river was muddy.';
  document.body.appendChild(p);
  return window.kalamSentenceAroundText('bank', p.firstChild) === 'The BANK of the river was muddy.';
})());

// Tap flow: stub caretFromPoint, zero the delay, click content.
window.kalamCaretFromPoint = () => ({node: firstText, offset: bankIdx + 2});
window.kalamSetTapDelay(0);
window.getSelection().removeAllRanges();
const tapPara = document.createElement('p');
tapPara.textContent = 'The bank was crowded this morning.';
document.body.appendChild(tapPara);
bridgeCalls = [];
tapPara.dispatchEvent(new window.MouseEvent('click', {bubbles: true, cancelable: true, clientX: 40, clientY: 40}));
setTimeout(() => {
  const tapLookup = bridgeCalls.find(m => m.type === 'dict-lookup' && m.word === 'bank');
  check('tap schedules dict-lookup with word', !!tapLookup && tapLookup.word === 'bank');
  check('tap sends the surrounding sentence as context',
    !!tapLookup && tapLookup.context === 'The children played on the muddy bank of the stream.');
  check('tap sends a rect for anchoring',
    !!(tapLookup && tapLookup.rect && tapLookup.rect.x === 12 && tapLookup.rect.y === 34));
  check('tap on a word does not close an open popup', popup.style.display === 'block');

  // Tap on empty space (caret resolves nothing) closes the popup
  window.kalamCaretFromPoint = () => null;
  window.kalamShowDict({word:'x', pos:[], senses:[{number:1,pos:null,def:'d',example:null}], synonyms:[], antonyms:[], idioms:[], suggestions:[], saved:false}, null);
  document.body.click();
  check('tap on empty space closes popup', popup.style.display === 'none');
  window.kalamCaretFromPoint = () => ({node: firstText, offset: 7});

  // ---- Phase 6: popup keyboard ----
  window.kalamShowDict({word:'focus', pos:['verb'], senses:[
    {number:1,pos:'verb',def:'first def',example:null},
    {number:2,pos:'verb',def:'second def',example:null}
  ], synonyms:[], antonyms:[], idioms:[], suggestions:[], saved:false}, null);
  const itemsF = popup.querySelectorAll('.k-def-item');
  check('fresh entry has no sense focus', popup.querySelectorAll('.k-def-focus').length === 0);
  window.dispatchEvent(new window.KeyboardEvent('keydown', {key:'ArrowDown', bubbles:true, cancelable:true}));
  check('ArrowDown focuses the first sense', itemsF[0].classList.contains('k-def-focus'));
  window.dispatchEvent(new window.KeyboardEvent('keydown', {key:'ArrowDown', bubbles:true, cancelable:true}));
  check('ArrowDown moves to the second sense', itemsF[1].classList.contains('k-def-focus') && !itemsF[0].classList.contains('k-def-focus'));
  window.dispatchEvent(new window.KeyboardEvent('keydown', {key:'ArrowUp', bubbles:true, cancelable:true}));
  check('ArrowUp moves back', itemsF[0].classList.contains('k-def-focus'));
  // ArrowDown from the last sense wraps to the first
  window.dispatchEvent(new window.KeyboardEvent('keydown', {key:'ArrowDown', bubbles:true, cancelable:true}));
  window.dispatchEvent(new window.KeyboardEvent('keydown', {key:'ArrowDown', bubbles:true, cancelable:true}));
  check('ArrowDown wraps to the first sense', itemsF[0].classList.contains('k-def-focus'));

  // Enter with a focused sense saves the word with that sense's definition
  // (focus wrapped back to the first sense)
  bridgeCalls = [];
  window.dispatchEvent(new window.KeyboardEvent('keydown', {key:'Enter', bubbles:true, cancelable:true}));
  check('Enter saves with the focused sense definition',
    bridgeCalls.some(m => m.type === 'save-word' && m.word === 'focus' && m.definition === 'first def'));
  check('Enter toggles the save button', document.getElementById('kalam-dict-save').classList.contains('saved'));
  // Header save with no focus uses the first sense
  window.kalamShowDict({word:'plain', pos:['verb'], senses:[
    {number:1,pos:'verb',def:'plain first',example:null},
    {number:2,pos:'verb',def:'plain second',example:null}
  ], synonyms:[], antonyms:[], idioms:[], suggestions:[], saved:false}, null);
  bridgeCalls = [];
  document.getElementById('kalam-dict-save').click();
  check('header save without focus uses the first sense',
    bridgeCalls.some(m => m.type === 'save-word' && m.word === 'plain' && m.definition === 'plain first'));
  // Enter with no focused sense does nothing
  window.kalamShowDict({word:'none', pos:[], senses:[{number:1,pos:null,def:'d',example:null}], synonyms:[], antonyms:[], idioms:[], suggestions:[], saved:false}, null);
  bridgeCalls = [];
  window.dispatchEvent(new window.KeyboardEvent('keydown', {key:'Enter', bubbles:true, cancelable:true}));
  check('Enter without focus does nothing', bridgeCalls.length === 0);

  // ArrowDown reveals hidden "Show N more" senses when focus lands there
  window.kalamShowDict({word:'many', pos:[], senses:Array.from({length:6}, (_, i) => ({number:i+1,pos:null,def:'sense '+(i+1),example:null})), synonyms:[], antonyms:[], idioms:[], suggestions:[], saved:false}, null);
  const extra = document.getElementById('kalam-extra-defs');
  check('extra senses start hidden', !extra.classList.contains('visible'));
  for (let i = 0; i < 6; i++) window.dispatchEvent(new window.KeyboardEvent('keydown', {key:'ArrowDown', bubbles:true, cancelable:true}));
  check('ArrowDown into hidden extras reveals them', extra.classList.contains('visible'));
  check('focus lands on the 6th sense', popup.querySelectorAll('.k-def-item')[5].classList.contains('k-def-focus'));

  // ---- Phase 6: find in chapter ----
  window.kalamShowDict({word:'bank', pos:['noun'], senses:[{number:1,pos:'noun',def:'a financial institution',example:null}], synonyms:[], antonyms:[], idioms:[], suggestions:[], saved:false}, null);
  const findBtn = document.getElementById('kalam-dict-find');
  bridgeCalls = [];
  findBtn.click();
  check('find button fires search-in-book', bridgeCalls.some(m => m.type === 'search-in-book' && m.word === 'bank'));

  const pFind = document.createElement('p');
  pFind.textContent = 'The bank manager opened a new bank account at the bank.';
  document.body.appendChild(pFind);
  bridgeCalls = [];
  window.kalamSearchInBook('bank');
  check('find wraps every occurrence', pFind.querySelectorAll('.kalam-search-hit').length === 3);
  // The count covers every occurrence in the whole chapter (all paragraphs),
  // which must equal the number of wrapped spans.
  check('find reports the count',
    bridgeCalls.some(m => m.type === 'search-in-book-done' &&
      m.count === document.querySelectorAll('.kalam-search-hit').length));
  check('find is case-insensitive', (() => {
    const p = document.createElement('p');
    p.textContent = 'BANK and bank.';
    document.body.appendChild(p);
    window.kalamSearchInBook('bank');
    const n = p.querySelectorAll('.kalam-search-hit').length;
    window.kalamClearSearchHits();
    return n === 2;
  })());
  check('clearSearchHits unwraps', pFind.querySelectorAll('.kalam-search-hit').length === 0);
  check('re-running find does not double-wrap', (() => {
    window.kalamSearchInBook('bank');
    window.kalamSearchInBook('bank');
    const n = pFind.querySelectorAll('.kalam-search-hit').length;
    window.kalamClearSearchHits();
    return n === 3;
  })());
  check('find skips highlighted annotation spans', (() => {
    const p = document.createElement('p');
    const hl = document.createElement('span');
    hl.className = 'kalam-hl';
    hl.textContent = 'bank bank';
    p.appendChild(hl);
    document.body.appendChild(p);
    window.kalamSearchInBook('bank');
    const inside = hl.querySelectorAll('.kalam-search-hit').length;
    window.kalamClearSearchHits();
    return inside === 0;
  })());
  check('Esc-equivalent clear works via kalamClearSearchHits', pFind.querySelectorAll('.kalam-search-hit').length === 0);

  console.log(failures === 0 ? '\nALL CHECKS PASSED' : `\n${failures} CHECKS FAILED`);
  process.exit(failures === 0 ? 0 : 1);
}, 30);
