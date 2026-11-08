// The example site's search: a plain substring match over `documents`, which Folia writes
// before this code (folia::search).
(function () {
    var input = document.getElementById("search-input");
    var results = document.getElementById("search-results");
    if (!input || !results) return;
    document.getElementById("search-form").addEventListener("submit", function (e) {
        e.preventDefault();
    });
    input.addEventListener("input", function () {
        var term = input.value.trim().toLowerCase();
        results.innerHTML = "";
        if (!term) return;
        documents.forEach(function (doc) {
            if ((doc.title + " " + doc.body).toLowerCase().indexOf(term) === -1) return;
            var item = document.createElement("li");
            var link = document.createElement("a");
            link.href = doc.url;
            link.textContent = doc.title;
            item.appendChild(link);
            results.appendChild(item);
        });
    });
})();
