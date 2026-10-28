const input = document.getElementById(inputId);
if (input) {
    // Number inputs can discard Arabic digits before Rust receives an input event.
    const insertArabicDigits = (event, text) => {
        if (!/[٠-٩]/u.test(text)) return;
        event.preventDefault();
        const ascii = text.replace(/[٠-٩]/gu, digit => String(digit.charCodeAt(0) - 0x0660));
        // Number inputs do not support setRangeText. This keeps selection and emits input.
        document.execCommand("insertText", false, ascii);
    };
    input.addEventListener("beforeinput", event => insertArabicDigits(event, event.data || ""));
    input.addEventListener("paste", event => insertArabicDigits(event, event.clipboardData?.getData("text") || ""));
    input.focus();
    input.select();
}
