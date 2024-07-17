import { FaSolidCheck } from "solid-icons/fa";
import { TbSelector } from "solid-icons/tb";
import { createEffect, createSignal, For, JSX, Show } from "solid-js";
import {
  DisclosureStateChild,
  Listbox,
  ListboxButton,
  ListboxOption,
  ListboxOptions,
} from "terracotta";
import createFuzzySearch from "@nozbe/microfuzz";

interface SelectProps<T> {
  options: T[];
  display: (option: T) => string;
  selected: T;
  onSelected: (option: T) => void;
  class?: string;
  label?: JSX.Element;
  id?: string;
}

export const Select = <T,>(props: SelectProps<T>) => {
  const [open, setOpen] = createSignal(false);
  const [searchTerm, setSearchTerm] = createSignal("");
  const [searchResults, setSearchResults] = createSignal<T[]>([]);

  createEffect(() => {
    if (searchTerm() === "") {
      setSearchResults(props.options);
    } else {
      const fuzzy = createFuzzySearch(props.options, {
        getText: (item: T) => {
          return [props.display(item)];
        },
      });
      const results = fuzzy(searchTerm());
      setSearchResults(results.map((result) => result.item));

      const input = document.getElementById(`${props.id}-search`);
      if (input) {
        setTimeout(() => {
          input.focus();
        }, 500);
        setTimeout(() => {
          input.focus();
        }, 1000);
      }
    }
  });

  return (
    <>
      <Show when={props.label}>{(label) => label()}</Show>
      <Listbox
        class={`bg-neutral-200/70 min-w-[100px] relative border rounded border-neutral-300 ${props.class}`}
        value={props.selected}
        defaultOpen={false}
        onClose={() => setSearchTerm("")}
      >
        <ListboxButton
          class="flex py-1 text-sm px-3 w-full justify-between gap-2 items-center"
          onClick={() => setOpen(!open())}
        >
          {props.display(props.selected)}
          <TbSelector />
        </ListboxButton>
        <DisclosureStateChild>
          {({ isOpen }): JSX.Element => (
            <Show when={isOpen()}>
              <div class="relative w-full">
                <ListboxOptions
                  unmount={false}
                  tabIndex={0}
                  class="absolute z-40 shadow mt-1 max-h-[70vh] w-full overflow-y-auto overflow-x-none rounded-md bg-white text-base outline outline-1 outline-gray-300 ring-1 ring-black ring-opacity-5 focus:outline-none sm:text-sm"
                >
                  <Show when={props.options.length > 5}>
                    <input
                      id={`${props.id}-search`}
                      placeholder="Search..."
                      class="mb-2 flex mx-auto items-center justify-between rounded bg-neutral-200 p-1 mt-2 text-sm text-black outline-none dark:bg-neutral-700 dark:hover:text-white dark:focus:text-white"
                      onInput={(e) => {
                        setSearchTerm(e.target.value);
                      }}
                      value={searchTerm()}
                    />
                  </Show>
                  <For each={searchResults()}>
                    {(option): JSX.Element => (
                      <ListboxOption
                        class="group min-w-full w-[max-content] rounded-md focus:outline-none"
                        value={option}
                      >
                        {({ isSelected }): JSX.Element => (
                          <div
                            classList={{
                              "bg-magenta-100 text-magenta-900": isSelected(),
                              "text-gray-900": !isSelected(),
                              "group-hover:bg-magenta-50 group-hover:cursor-pointer whitespace-nowrap flex p-2 justify-between items-center gap-2 group-hover:text-magenta-900 relative cursor-default select-none ":
                                true,
                            }}
                            onClick={() => {
                              props.onSelected(option);
                              setOpen(false);
                            }}
                          >
                            <span>{props.display(option)}</span>
                            {isSelected() ? (
                              <span
                                classList={{
                                  "": true,
                                }}
                              >
                                <FaSolidCheck />
                              </span>
                            ) : null}
                          </div>
                        )}
                      </ListboxOption>
                    )}
                  </For>
                </ListboxOptions>
              </div>
            </Show>
          )}
        </DisclosureStateChild>
      </Listbox>
    </>
  );
};
