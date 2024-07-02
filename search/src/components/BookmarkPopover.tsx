import {
  For,
  Setter,
  Show,
  createEffect,
  createSignal,
  useContext,
} from "solid-js";
import {
  Menu,
  MenuItem,
  Popover,
  PopoverButton,
  PopoverPanel,
} from "solid-headless";
import { RiSystemAddFill } from "solid-icons/ri";
import {
  isChunkGroupPageDTO,
  type ChunkBookmarksDTO,
  type ChunkGroupDTO,
  type ChunkMetadata,
} from "../../utils/apiTypes";
import InputRowsForm from "./Atoms/InputRowsForm";
import { VsBookmark } from "solid-icons/vs";
import { BiRegularChevronLeft, BiRegularChevronRight } from "solid-icons/bi";
import { A } from "@solidjs/router";
import { DatasetAndUserContext } from "./Contexts/DatasetAndUserContext";

export interface BookmarkPopoverProps {
  chunkMetadata: ChunkMetadata;
  chunkGroups: ChunkGroupDTO[];
  totalGroupPages: number;
  setLoginModal?: Setter<boolean>;
  bookmarks: ChunkBookmarksDTO[];
  setChunkGroups?: Setter<ChunkGroupDTO[]>;
}

const BookmarkPopover = (props: BookmarkPopoverProps) => {
  const apiHost = import.meta.env.VITE_API_HOST as string;
  const datasetAndUserContext = useContext(DatasetAndUserContext);
  const $currentUser = datasetAndUserContext.user;
  const $dataset = datasetAndUserContext.currentDataset;

  const [refetchingChunkGroups, setRefetchingChunkGroups] = createSignal(false);
  const [refetchingBookmarks, setRefetchingBookmarks] = createSignal(false);
  const [showGroupForm, setShowGroupForm] = createSignal(false);
  const [notLoggedIn, setNotLoggedIn] = createSignal(false);
  const [groupFormTitle, setGroupFormTitle] = createSignal("");
  const [usingPanel, setUsingPanel] = createSignal(false);
  const [bookmarks, setBookmarks] = createSignal<ChunkBookmarksDTO[]>([]);
  const [localGroupPage, setLocalGroupPage] = createSignal(1);
  const [localChunkGroups, setLocalChunkGroups] = createSignal<ChunkGroupDTO[]>(
    [],
  );

  createEffect(() => {
    const groupsToAdd: ChunkGroupDTO[] = [];
    props.bookmarks.forEach((b) => {
      b.slim_groups.forEach((c) => {
        groupsToAdd.push({
          id: c.id,
          name: c.name,
          description: "",
          created_at: "",
          updated_at: "",
        });
      });
    });

    setBookmarks(props.bookmarks);
    setLocalChunkGroups([...groupsToAdd, ...props.chunkGroups]);
  });

  createEffect((prevPage) => {
    const curPage = localGroupPage();
    if (curPage == prevPage) {
      return curPage;
    }

    const chunkBookmarks = bookmarks();
    const setChunkGroups = props.setChunkGroups;
    refetchGroups(curPage, chunkBookmarks, setChunkGroups);

    return curPage;
  }, 1);

  createEffect(() => {
    if ($currentUser?.()?.id === undefined) {
      return;
    }
    if (!refetchingChunkGroups()) {
      return;
    }

    const curPage = localGroupPage();
    const chunkBookmarks = bookmarks();
    const setChunkGroups = props.setChunkGroups;
    refetchGroups(curPage, chunkBookmarks, setChunkGroups);
    setRefetchingChunkGroups(false);
  });

  createEffect(() => {
    if ($currentUser?.()?.id === undefined) {
      return;
    }
    if (!refetchingBookmarks()) {
      return;
    }

    const curGroupPage = localGroupPage();
    refetchBookmarks(curGroupPage);
    setRefetchingBookmarks(false);
  });

  const refetchGroups = (
    curPage: number,
    chunkBookmarks: ChunkBookmarksDTO[],
    setChunkGroups: Setter<ChunkGroupDTO[]> | undefined,
  ) => {
    const currentDataset = $dataset?.();
    if (!currentDataset) return;

    void fetch(
      `${apiHost}/dataset/groups/${
        currentDataset.dataset.id
      }/${localGroupPage()}`,
      {
        method: "GET",
        headers: {
          "TR-Dataset": currentDataset.dataset.id,
        },
        credentials: "include",
      },
    ).then((response) => {
      if (!setChunkGroups) return;

      if (response.ok) {
        void response.json().then((data) => {
          if (isChunkGroupPageDTO(data)) {
            if (curPage !== 1) {
              setLocalChunkGroups(data.groups);
              return;
            }

            const groupsToAdd: ChunkGroupDTO[] = [];

            chunkBookmarks.forEach((chunkBookmark) => {
              chunkBookmark.slim_groups.forEach((group) => {
                const chunkGroup: ChunkGroupDTO = {
                  id: group.id,
                  name: group.name,
                  description: "",
                  created_at: "",
                  updated_at: "",
                };

                groupsToAdd.push(chunkGroup);
              });
            });

            const deDupedPrev = data.groups.filter((group) => {
              return (
                groupsToAdd.find((groupToAdd) => groupToAdd.id == group.id) ==
                undefined
              );
            });

            const updatedGroups = [...groupsToAdd, ...deDupedPrev];
            setLocalChunkGroups(updatedGroups);
            setChunkGroups(updatedGroups);
          }
        });
      }

      if (response.status == 401) {
        setNotLoggedIn(true);
      }
    });
  };

  const refetchBookmarks = (curPage: number) => {
    const currentDataset = $dataset?.();
    if (!currentDataset) return;

    void fetch(`${apiHost}/chunk_group/chunks`, {
      method: "POST",
      credentials: "include",
      headers: {
        "Content-Type": "application/json",
        "TR-Dataset": currentDataset.dataset.id,
      },
      body: JSON.stringify({
        chunk_ids: [props.chunkMetadata.id],
      }),
    }).then((response) => {
      if (response.ok) {
        void response.json().then((data) => {
          const chunkBookmarks = data as ChunkBookmarksDTO[];

          setBookmarks(chunkBookmarks);

          if (curPage !== 1) {
            return;
          }

          const groupsToAdd: ChunkGroupDTO[] = [];

          chunkBookmarks.forEach((chunkBookmark) => {
            chunkBookmark.slim_groups.forEach((group) => {
              const chunkGroup: ChunkGroupDTO = {
                id: group.id,
                name: group.name,
                description: "",
                created_at: "",
                updated_at: "",
              };

              groupsToAdd.push(chunkGroup);
            });
          });

          setLocalChunkGroups((prev) => {
            const deDupedPrev = prev.filter((group) => {
              return (
                groupsToAdd.find((groupToAdd) => groupToAdd.id == group.id) ==
                undefined
              );
            });

            return [...groupsToAdd, ...deDupedPrev];
          });
        });
      }
    });
  };

  return (
    <Popover defaultOpen={false} class="relative">
      {({ isOpen, setState }) => (
        <div>
          <div class="flex items-center">
            <PopoverButton
              title="Bookmark"
              onClick={() => {
                if (notLoggedIn() || $currentUser?.()?.id === undefined) {
                  props.setLoginModal?.(true);
                  return;
                }
                refetchBookmarks(localGroupPage());
              }}
            >
              <VsBookmark class="z-0 h-5 w-5 fill-current" />
            </PopoverButton>
          </div>
          <Show
            when={
              (isOpen() || usingPanel()) &&
              !notLoggedIn() &&
              !($currentUser?.()?.id === undefined)
            }
          >
            <PopoverPanel
              unmount={false}
              class="absolute z-50 w-screen max-w-xs -translate-x-[300px] translate-y-1"
              onMouseEnter={() => setUsingPanel(true)}
              onMouseLeave={() => setUsingPanel(false)}
              onClick={() => setState(true)}
            >
              <Menu class=" flex w-full flex-col justify-end space-y-2 overflow-hidden rounded bg-white py-4 shadow-2xl dark:bg-shark-700">
                <div class="mb-3 w-full px-4 text-center text-lg font-bold">
                  Manage Groups For This Chunk
                </div>
                <MenuItem as="button" aria-label="Empty" />
                <div class="max-w-screen mx-1 max-h-[20vh] transform justify-end space-y-2 overflow-y-auto rounded px-4 scrollbar-thin scrollbar-track-neutral-200 scrollbar-thumb-neutral-600 scrollbar-track-rounded-md scrollbar-thumb-rounded-md dark:scrollbar-track-neutral-700 dark:scrollbar-thumb-neutral-400">
                  <For each={localChunkGroups()}>
                    {(group, idx) => {
                      return (
                        <>
                          <Show when={idx() != 0}>
                            <div class="h-px w-full bg-neutral-200 dark:bg-neutral-700" />
                          </Show>
                          <div class="flex w-full items-center justify-between space-x-2">
                            <A
                              href={`/group/${group.id}`}
                              class="max-w-[80%] break-all underline"
                            >
                              {group.name}
                            </A>

                            <input
                              type="checkbox"
                              checked={
                                bookmarks().find((bookmark) =>
                                  bookmark.slim_groups
                                    .map((slimGroup) => slimGroup.id)
                                    .includes(group.id),
                                )
                                  ? true
                                  : false
                              }
                              onChange={(e) => {
                                const currentDataset = $dataset?.();
                                if (!currentDataset) return;
                                void fetch(
                                  `${apiHost}/chunk_group/chunk/${group.id}`,
                                  {
                                    method: e.currentTarget.checked
                                      ? "POST"
                                      : "DELETE",
                                    headers: {
                                      "Content-Type": "application/json",
                                      "TR-Dataset": currentDataset.dataset.id,
                                    },
                                    body: JSON.stringify({
                                      chunk_id: props.chunkMetadata.id,
                                    }),
                                    credentials: "include",
                                  },
                                ).then((response) => {
                                  if (!response.ok) {
                                    e.currentTarget.checked =
                                      !e.currentTarget.checked;
                                  }
                                  setRefetchingBookmarks(true);
                                });
                                setState(true);
                              }}
                              class="h-4 w-4 cursor-pointer rounded-sm border-gray-300 bg-neutral-500 accent-turquoise focus:ring-neutral-200 dark:border-neutral-700 dark:focus:ring-neutral-600"
                            />
                          </div>
                        </>
                      );
                    }}
                  </For>
                  <div class="flex items-center justify-between">
                    <div />
                    <div class="flex items-center">
                      <div class="text-sm text-neutral-400">
                        {localGroupPage()} /{" "}
                        {props.totalGroupPages == 0 ? 1 : props.totalGroupPages}
                      </div>
                      <button
                        class="disabled:text-neutral-400 dark:disabled:text-neutral-500"
                        disabled={localGroupPage() == 1}
                        onClick={() => {
                          setState(true);
                          setLocalGroupPage((prev) => prev - 1);
                        }}
                      >
                        <BiRegularChevronLeft class="h-6 w-6 fill-current" />
                      </button>
                      <button
                        class="disabled:text-neutral-400 dark:disabled:text-neutral-500"
                        disabled={
                          localGroupPage() ==
                          (props.totalGroupPages == 0
                            ? 1
                            : props.totalGroupPages)
                        }
                        onClick={() => {
                          setState(true);
                          setLocalGroupPage((prev) => prev + 1);
                        }}
                      >
                        <BiRegularChevronRight class="h-6 w-6 fill-current" />
                      </button>
                    </div>
                  </div>
                </div>
                <Show when={showGroupForm()}>
                  <div class="mx-4 rounded bg-gray-100 py-2 dark:bg-neutral-800">
                    <div class="px-2 text-lg font-bold">Create New Group</div>
                    <div>
                      <InputRowsForm
                        createButtonText="Create group"
                        onCreate={() => {
                          const title = groupFormTitle();
                          if (title.trim() == "") return;
                          const currentDataset = $dataset?.();
                          if (!currentDataset) return;
                          void fetch(`${apiHost}/chunk_group`, {
                            method: "POST",
                            headers: {
                              "Content-Type": "application/json",
                              "TR-Dataset": currentDataset.dataset.id,
                            },
                            credentials: "include",
                            body: JSON.stringify({
                              name: title,
                              description: "",
                            }),
                          }).then(() => {
                            setRefetchingChunkGroups(true);
                            setShowGroupForm(false);
                            setGroupFormTitle("");
                            setState(true);
                          });
                        }}
                        onCancel={() => {
                          setShowGroupForm(false);
                          setState(true);
                        }}
                        inputGroups={[
                          {
                            label: "Title",
                            inputValue: groupFormTitle,
                            setInputValue: setGroupFormTitle,
                          },
                        ]}
                      />
                    </div>
                  </div>
                </Show>
                {!showGroupForm() && (
                  <div class="px-4 pt-4">
                    <MenuItem
                      as="button"
                      onClick={() => {
                        setShowGroupForm(true);
                        setState(true);
                      }}
                      class="flex w-full items-center justify-center rounded-full border border-green-500 bg-transparent px-2 text-lg text-green-500"
                    >
                      <RiSystemAddFill class="h-5 w-5 fill-current" />
                      <p> Create New Group </p>
                    </MenuItem>
                  </div>
                )}
              </Menu>
            </PopoverPanel>
          </Show>
        </div>
      )}
    </Popover>
  );
};

export default BookmarkPopover;
