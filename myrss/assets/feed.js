let tz = "timezone=" + new Date().getTimezoneOffset();
document.cookie = tz;

var eventSource = new EventSource("/stream");
function getCookie(name) {
    const value = `; ${document.cookie}`;
    const parts = value.split(`; ${name}=`);
    if (parts.length === 2) return parts.pop().split(';').shift();
}
eventSource.onmessage = function(event) { // console.log("appending message: ");
    // console.log(event.data);
    let parsedData = JSON.parse(event.data);
    let sender = parsedData.sender;
    let message = parsedData.message;
    let preview = parsedData.preview;
    let notify = parsedData.notify;

    document.getElementById("messages").insertAdjacentHTML("afterbegin", message);

    // Check if the browser supports notifications
    if (sender != getCookie("sender-name") && "Notification" in window && notify) {
        if (Notification.permission === "granted" && !document.hasFocus()) {
            // Create the notification
            var notification = new Notification("Message from " + sender, {
                body: preview,
            });
        }
    }
};
eventSource.onerror = function() {
    console.error("Error occurred in SSE connection. Trying to reconnect.");
    eventSource = new EventSource("/stream");
};

window.addEventListener("beforeunload", (event) => {
    if (eventSource) {
        eventSource.close();
        console.log("SSE connection closed");
    }
});
function togglePickerOpen() {
    let pickerDialog = document.getElementById("pickerDialog");
    if (pickerDialog.open) {
        pickerDialog.close();
        document.getElementById("message-input").focus();
    } else { pickerDialog.showModal();
    }
}
document.addEventListener("DOMContentLoaded", () => {
    document.querySelector('emoji-picker')
        .addEventListener('emoji-click', function(event) {
            document.getElementById("message-input").value += event.detail.unicode;
            togglePickerOpen();
        });
    document.getElementById("emoji-icon").addEventListener("click", function(event) {
        event.preventDefault()
        togglePickerOpen();
    });
});
