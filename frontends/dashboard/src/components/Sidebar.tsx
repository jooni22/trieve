import { For, createMemo, createSignal, useContext } from "solid-js";
import { UserContext } from "../contexts/UserContext";
import { useNavigate } from "@solidjs/router";
import { IoLogOutOutline, IoOpenOutline } from "solid-icons/io";
import { AiOutlinePlus, AiOutlineUser } from "solid-icons/ai";
import CreateNewOrgModal from "./CreateNewOrgModal";
import { DatasetContext } from "../contexts/DatasetContext";

export const Sidebar = () => {
  const apiHost = import.meta.env.VITE_API_HOST as string;
  const analyticsUiURL = import.meta.env.VITE_ANALYTICS_UI_URL as string;
  const searchUiURL = import.meta.env.VITE_SEARCH_UI_URL as string;
  const chatUiURL = import.meta.env.VITE_CHAT_UI_URL as string;

  const navigate = useNavigate();

  const userContext = useContext(UserContext);
  const datasetContext = useContext(DatasetContext);

  const [showNewOrgModal, setShowNewOrgModal] = createSignal(false);

  const organizations = createMemo(() => userContext?.user?.()?.orgs ?? []);

  // Construct ?organization=7136f468-fedb-4b50-a689-cbf94e01d629&dataset=674c7d49-132e-4d94-8b0e-3c8f898eda49 URL
  const orgDatasetParams = createMemo(() => {
    const orgId = userContext.selectedOrganizationId?.();
    const datasetId = datasetContext.dataset?.()?.id;
    let params = "";
    if (orgId) params += `?organization=${orgId}`;
    if (orgId && datasetId) params += `&dataset=${datasetId}`;
    return params;
  });

  return (
    <>
      <div class="flex min-h-[calc(100vh-65px)] w-full max-w-[280px] flex-col justify-between border-r bg-neutral-100">
        <div class="sticky top-0 flex flex-col">
          <div class="border-b px-4 py-3">
            <h5 class="font-semibold text-neutral-600">Organizations</h5>
            <div class="flex flex-col items-start space-y-1 py-2">
              <For each={organizations()}>
                {(org) => (
                  <button
                    onClick={() => {
                      userContext.setSelectedOrganizationId(org.id);
                      navigate(`/dashboard/${org.id}/overview`);
                    }}
                    classList={{
                      "block hover:text-fuchsia-800": true,
                      "text-magenta":
                        userContext.selectedOrganizationId?.() === org.id,
                    }}
                  >
                    {org.name}
                  </button>
                )}
              </For>
            </div>
            <button
              class="flex items-center gap-2 rounded-md border border-neutral-300 bg-white px-2 py-1 text-sm hover:border-fuchsia-800 hover:text-fuchsia-800"
              onClick={() => setShowNewOrgModal(true)}
            >
              <AiOutlinePlus class="inline-block h-4 w-4" />{" "}
              <p>New Organization</p>
            </button>
          </div>
          <div class="border-b px-4 py-3">
            <h5 class="font-semibold text-neutral-600">Admin Tools</h5>
            <div class="flex flex-col items-start space-y-1 py-2">
              <div class="flex items-center text-neutral-800 hover:text-fuchsia-800">
                <a
                  href={`${searchUiURL}${orgDatasetParams()}`}
                  target="_blank"
                  class="flex items-center"
                >
                  Search playground{" "}
                  <IoOpenOutline class="ml-1 inline-block h-4 w-4" />
                </a>
              </div>
              <div class="flex items-center text-neutral-800 hover:text-fuchsia-800">
                <a
                  href={`${chatUiURL}${orgDatasetParams()}`}
                  target="_blank"
                  class="flex items-center"
                >
                  <span>RAG playground</span>{" "}
                  <IoOpenOutline class="ml-1 inline-block h-4 w-4" />
                </a>
              </div>
              <div class="flex items-center text-neutral-800 hover:text-fuchsia-800">
                <a
                  href={`${analyticsUiURL}${orgDatasetParams()}`}
                  target="_blank"
                  class="flex items-center"
                >
                  Analytics playground{" "}
                  <IoOpenOutline class="ml-1 inline-block h-4 w-4" />
                </a>
              </div>
            </div>
          </div>
        </div>
        <div class="sticky bottom-0 flex flex-col items-start border-t px-4 py-4">
          <div class="flex items-center gap-2">
            <p>{userContext?.user?.()?.email}</p>
            <AiOutlineUser class="h-4 w-4" />
          </div>
          <button
            class="flex items-center gap-2 hover:text-fuchsia-800"
            onClick={() => {
              void fetch(`${apiHost}/auth?redirect_uri=${window.origin}`, {
                method: "DELETE",
                credentials: "include",
              }).then((res) => {
                res
                  .json()
                  .then((res) => {
                    // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment, @typescript-eslint/no-unsafe-member-access
                    window.location.href = res.logout_url;
                  })
                  .catch(() => {
                    console.log("error");
                  });
              });
            }}
          >
            Log Out <IoLogOutOutline class="inline-block h-4 w-4" />
          </button>
        </div>
      </div>
      <CreateNewOrgModal
        isOpen={showNewOrgModal}
        closeModal={() => setShowNewOrgModal(false)}
      />
    </>
  );
};
