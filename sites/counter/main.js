document.addEventListener("DOMContentLoaded", function () {
	const counter = document.querySelector("#counter");

	document.querySelector("#plus").addEventListener("click", function () {
		counter.textContent = parseInt(counter.textContent) + 1;
	});

	document.querySelector("#minus").addEventListener("click", function () {
		counter.textContent = parseInt(counter.textContent) - 1;
	});
});
