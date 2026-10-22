<script lang="ts">
  import type { FieldDef } from "$lib/api/types";

  let {
    field,
    value = $bindable(""),
    savedSecret = false,
    disabled = false,
  }: {
    field: FieldDef;
    value: string;
    savedSecret?: boolean;
    disabled?: boolean;
  } = $props();

  const inputId = $derived(`field-${field.key}`);
  const isSecret = $derived(field.kind.kind === "secret");
  const placeholder = $derived(
    isSecret && savedSecret ? "Saved. Leave blank to keep it." : field.required ? "" : "Optional",
  );
</script>

<div class="field">
  <label for={inputId}>
    {field.label}{#if field.required}<span class="required" aria-hidden="true"> *</span>{/if}
  </label>

  {#if field.kind.kind === "choice"}
    <select id={inputId} bind:value {disabled}>
      {#each field.kind.options as option (option.value)}
        <option value={option.value}>{option.label}</option>
      {/each}
    </select>
  {:else if field.kind.kind === "number"}
    <input id={inputId} type="number" inputmode="numeric" min={field.kind.min} max={field.kind.max} bind:value {disabled} {placeholder} />
  {:else}
    <input
      id={inputId}
      type={isSecret ? "password" : "text"}
      autocomplete={isSecret ? "new-password" : "off"}
      spellcheck="false"
      bind:value
      {disabled}
      {placeholder}
    />
  {/if}

  {#if field.help}
    <p class="help">{field.help}</p>
  {/if}
</div>

<style>
  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-bottom: 14px;
  }
  label {
    font-weight: 500;
  }
  .required {
    color: var(--text-secondary);
  }
  input,
  select {
    background: var(--control-bg);
    border: 1px solid transparent;
    border-radius: var(--radius-control);
    padding: 6px 9px;
    min-width: 0;
    user-select: text;
  }
  input:focus-visible,
  select:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 0;
  }
  input::placeholder {
    color: var(--text-secondary);
  }
  .help {
    margin: 0;
    color: var(--text-secondary);
    font-size: 12px;
  }
</style>
