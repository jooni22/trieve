import { createQuery } from "@tanstack/solid-query";
import { DatasetContext } from "../layouts/TopBarLayout";
import { getQueriesForTopic, getTrendsBubbles } from "../api/trends";
import { createSignal, For, Show, useContext } from "solid-js";
import { TrendExplorerCanvas } from "../components/trend-explorer/TrendExplorerCanvas";
import { SearchQueryEvent } from "shared/types";

export const TrendExplorer = () => {
  const dataset = useContext(DatasetContext);

  const trendsQuery = createQuery(() => ({
    queryKey: ["trends", { dataset: dataset().dataset.id }],
    queryFn: async () => {
      return getTrendsBubbles(dataset().dataset.id);
    },
  }));

  const [selectedTopicId, setSelectedTopicId] = createSignal<string | null>(
    null,
  );

  const selectedTopicQuery = createQuery(() => ({
    queryKey: ["selected-topic", selectedTopicId()],
    queryFn: async () => {
      const selectedTopic = selectedTopicId();
      if (selectedTopic === null) {
        return [];
      }
      return getQueriesForTopic(dataset().dataset.id, selectedTopic);
    },
    enabled() {
      return selectedTopicId() !== null;
    },
  }));

  return (
    <div class="grid grow grid-cols-[300px_1fr]">
      <div class="border-r border-r-neutral-400 bg-neutral-200 p-2">
        <div class="flex flex-col gap-2">
          <For each={selectedTopicQuery?.data}>
            {(query) => <QueryCard searchEvent={query} />}
          </For>
        </div>
      </div>
      <Show when={trendsQuery?.data}>
        {(trends) => (
          <TrendExplorerCanvas
            onSelectTopic={(topic) => setSelectedTopicId(topic)}
            topics={trends()}
          />
        )}
      </Show>
    </div>
  );
};

interface QueryCardProps {
  searchEvent: SearchQueryEvent;
}
const QueryCard = (props: QueryCardProps) => {
  return (
    <div class="bg-white p-3">
      <div>{props.searchEvent.query}</div>
      <div>Score: {props.searchEvent.top_score}</div>
    </div>
  );
};
