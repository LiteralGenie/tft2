<script lang="ts">
    import { sleep } from 'radash'
    import FilterDialog from '../filter-dialog/filter-dialog.svelte'
    import type { SlotIndex } from '../filter-dialog/slot-tabs.svelte'
    import { getFilterFormContext } from '../form-context/context'
    import type { SlotFilter } from '../form-context/types'
    import {
        applyAttributeFilterWithGlobal,
        applyGlobalFilter
    } from '../form-context/utils'
    import FilterButton from './filter-button/filter-button.svelte'
    import FilterPreview from './filter-button/filter-preview.svelte'
    import GlobalFilterButton from './global-filter-button/global-filter-button.svelte'
    import GlobalFilterPreview from './global-filter-button/global-filter-preview.svelte'
    import TeamSizeInput from './team-size-input.svelte'

    let showDialog = false
    let activeSlotIndex: SlotIndex = 0

    const { form, submit } = getFilterFormContext()

    function handleDialogOpen(idx: SlotIndex) {
        activeSlotIndex = idx
        showDialog = true
    }

    async function handleDialogClose() {
        showDialog = false

        // Let dialog's close animation finish
        await sleep(250)

        submit()
    }

    function getSlotState(slot: SlotFilter): string {
        const numTotal = applyGlobalFilter($form.global).size

        if (slot.useAttributes) {
            const n = applyAttributeFilterWithGlobal(
                $form.global,
                slot.byAttribute
            ).size

            if (n === 0) {
                return 'error'
            } else if (n === 1) {
                // showing champion icon is possible but looks janky due to how large the container is + how zoomed in the image is
                // return matches.values().next().value
                return 'active'
            } else if (n === numTotal) {
                return 'inactive'
            } else {
                return 'active'
            }
        } else {
            const n = slot.byChampion.champions.filter(
                (c) => c.included
            ).length

            if (n === 1) {
                // return slot.byId[0].id
                return 'active'
            } else if (n > 1) {
                return 'active'
            } else {
                return 'inactive'
            }
        }
    }
</script>

<div class="root">
    <FilterDialog
        open={showDialog}
        slotIndex={activeSlotIndex}
        on:close={handleDialogClose}
        on:tabclick={(ev) => handleDialogOpen(ev.detail)}
    />

    <div class="card w-full rounded-sm flex flex-col justify-center">
        <div class="p-2 flex flex-col gap-2">
            <div class="flex flex-col gap-2">
                <div
                    class="filter-grid text-sm text-muted-foreground"
                >
                    {#each $form.slots.slice(0, $form.teamSize) as slot, idx}
                        <div class="cell flex gap-4 items-center">
                            <FilterButton
                                on:click={() => handleDialogOpen(idx)}
                                variant={getSlotState(slot)}
                            />
                            {#if getSlotState(slot) !== 'error'}
                                <FilterPreview {slot} />
                            {:else}
                                <span>
                                    No champions match the configured
                                    filters
                                </span>
                            {/if}
                        </div>
                    {/each}
                </div>
            </div>

            <div
                class="cell !p-1 text-muted-foreground text-sm flex flex-col sm:flex-row justify-end items-center gap-2 sm:gap-8"
            >
                <div class="w-full flex justify-start gap-2">
                    <GlobalFilterButton
                        on:click={() => handleDialogOpen('global')}
                    />

                    <GlobalFilterPreview />
                </div>

                <div class="w-full sm:w-max">
                    <TeamSizeInput />
                </div>
            </div>
        </div>
    </div>
</div>

<style lang="postcss">
    .card {
        background-color: hsl(var(--card) / 60%);
    }

    .filter-grid {
        display: grid;
        gap: 8px;
        grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    }

    .cell {
        @apply p-4 pl-8 border rounded-md;
        background-color: hsl(var(--foreground) / 8%);
    }

    .root :global(.settings-button) {
        background-color: hsl(var(--background) / 90%);
    }
</style>
