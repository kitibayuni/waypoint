<script lang="ts">
	import { uploadAttachment, deleteAttachment, downloadAttachmentUrl } from '$lib/api/attachments';
	import type { Attachment } from '$lib/api/attachments';

	let {
		engagementId,
		subjectType,
		subjectId,
		attachments = $bindable()
	}: {
		engagementId: string;
		subjectType: string;
		subjectId: string;
		attachments: Attachment[];
	} = $props();

	let fileInput = $state<HTMLInputElement | undefined>();
	let caption = $state('');
	let uploading = $state(false);
	let error = $state('');

	async function handleUpload() {
		const file = fileInput?.files?.[0];
		if (!file) return;
		uploading = true;
		error = '';
		try {
			const attachment = await uploadAttachment(
				engagementId,
				subjectType,
				subjectId,
				file,
				caption || undefined
			);
			attachments = [...attachments, attachment];
			caption = '';
			if (fileInput) fileInput.value = '';
		} catch {
			error = 'Upload failed.';
		} finally {
			uploading = false;
		}
	}

	async function handleDelete(id: string) {
		try {
			await deleteAttachment(id);
			attachments = attachments.filter((a) => a.id !== id);
		} catch {
			error = 'Failed to remove attachment.';
		}
	}

	function isImage(mime: string | null): boolean {
		return !!mime && mime.startsWith('image/');
	}
</script>

<div class="attachments">
	{#if error}<p class="error">{error}</p>{/if}

	<div class="gallery">
		{#each attachments as attachment (attachment.id)}
			<div class="attachment-card">
				{#if isImage(attachment.mime)}
					<a href={downloadAttachmentUrl(attachment.id)} target="_blank" rel="noopener">
						<img
							src={downloadAttachmentUrl(attachment.id)}
							alt={attachment.caption ?? attachment.filename}
						/>
					</a>
				{:else}
					<a
						href={downloadAttachmentUrl(attachment.id)}
						target="_blank"
						rel="noopener"
						class="file-link"
					>
						{attachment.filename}
					</a>
				{/if}
				<div class="meta">
					{attachment.filename}
					{#if attachment.caption}<br /><em>{attachment.caption}</em>{/if}
				</div>
				<button onclick={() => handleDelete(attachment.id)}>Remove</button>
			</div>
		{/each}
	</div>

	<div class="upload-form">
		<input type="file" bind:this={fileInput} />
		<input bind:value={caption} placeholder="Caption (optional)" />
		<button onclick={handleUpload} disabled={uploading}>Upload</button>
	</div>
</div>

<style>
	.error {
		color: #c0392b;
	}
	.gallery {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(10rem, 1fr));
		gap: 0.75rem;
		margin-bottom: 1rem;
	}
	.attachment-card {
		border: 1px solid #ddd;
		border-radius: 6px;
		padding: 0.5rem;
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
	}
	.attachment-card img {
		width: 100%;
		height: 6rem;
		object-fit: cover;
		border-radius: 4px;
	}
	.file-link {
		display: block;
		padding: 0.5rem;
		background: #f5f5f5;
		border-radius: 4px;
		text-align: center;
		word-break: break-all;
	}
	.meta {
		font-size: 0.8rem;
		color: #555;
		word-break: break-all;
	}
	.upload-form {
		display: flex;
		gap: 0.5rem;
		align-items: center;
		flex-wrap: wrap;
	}
</style>
