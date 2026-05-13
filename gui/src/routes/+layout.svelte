<script lang="ts">
	import '../app.css';
	let { children } = $props();
</script>

<svelte:head>
	<title>KoalaTalk</title>
</svelte:head>

{@render children()}
