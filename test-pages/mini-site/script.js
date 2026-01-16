// Mini Site JavaScript

console.log("Mini Site script loaded!");

// Click handler for the paragraph
var clickTarget = document.getElementById("click-target");
if (clickTarget) {
    clickTarget.addEventListener("click", function() {
        console.log("Click target was clicked!");
        var result = document.getElementById("result");
        if (result) {
            result.textContent = "You clicked the paragraph!";
        }
    });
}

// Change color button
var colorButton = document.getElementById("change-color");
if (colorButton) {
    colorButton.addEventListener("click", function() {
        console.log("Change color button clicked!");
        var intro = document.getElementById("intro");
        if (intro) {
            intro.setAttribute("style", "background-color: #e74c3c; color: white;");
        }
    });
}

// Add text button
var addButton = document.getElementById("add-text");
if (addButton) {
    addButton.addEventListener("click", function() {
        console.log("Add text button clicked!");
        var features = document.getElementById("features");
        if (features) {
            var newPara = document.createElement("p");
            newPara.textContent = "This paragraph was added by JavaScript!";
            features.appendChild(newPara);
        }
    });
}

console.log("Event listeners attached!");
