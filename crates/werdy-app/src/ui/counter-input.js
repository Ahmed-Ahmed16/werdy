const input = document.getElementById(inputId);
if (input) {
    // The current value starts selected, so the first typed digit replaces it.
    let replace = true;
    // Number inputs can discard Arabic digits before Rust receives an input event.
    const insertArabicDigits = (event, text) => {
        if (!/[٠-٩]/u.test(text)) {
            replace = false;
            return;
        }
        event.preventDefault();
        const ascii = text.replace(/[٠-٩]/gu, digit => String(digit.charCodeAt(0) - 0x0660));
        // Number inputs do not support setRangeText. This keeps selection and emits input.
        if (!document.execCommand("insertText", false, ascii)) {
            // Some WebViews disable insertText. Number inputs hide the caret
            // position, so replace the selected value once, then append.
            input.value = replace ? ascii : input.value + ascii;
            input.dispatchEvent(new Event("input", { bubbles: true }));
        }
        replace = false;
    };
    input.addEventListener("beforeinput", event => insertArabicDigits(event, event.data || ""));
    input.addEventListener("paste", event => insertArabicDigits(event, event.clipboardData?.getData("text") || ""));
    input.focus();
    input.select();
}
