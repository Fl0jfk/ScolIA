'use client';

import { useState, useEffect, useRef, useMemo } from "react";
import { useUser } from "@clerk/nextjs";
import Image from "next/image";
import DashboardThemeRoot from "@/app/components/Dashboard/DashboardThemeRoot";
import ReplayModuleTourButton from "@/app/components/module-tour/ReplayModuleTourButton";
import { dash } from "@/app/lib/dashboard-brand";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";

type Channel = {
  id: string;
  name: string;
  type: "public" | "restricted" | "private";
  roleRequired?: string;
  members?: string[];
  creatorId?: string;
};

type Message = {
  id: number;
  channel: string;
  content: string;
  createdAt: string;
  authorName: string;
  authorId: string | null;
  avatar: string | null;
};

type ClerkUser = {
  id: string;
  name: string;
  avatar: string;
};

const AUTHORIZED_ADMINS = ["HACQUEVILLE-MATHI", "DONA", "DUMOUCHEL","PLANTEC", "LEBLOND"];

export default function ProfChatPage() {
  const { user, isLoaded } = useUser();
  const [channels, setChannels] = useState<Channel[]>([]);
  const [activeChannel, setActiveChannel] = useState("general");
  const [messages, setMessages] = useState<Message[]>([]);
  const [text, setText] = useState("");
  const [loading, setLoading] = useState(true);
  const [showModal, setShowModal] = useState(false);
  const [showMembersModal, setShowMembersModal] = useState(false);
  const [newChanName, setNewChanName] = useState("");
  const [allUsers, setAllUsers] = useState<ClerkUser[]>([]);
  const [selectedMembers, setSelectedMembers] = useState<string[]>([]);
  const [isPrivate, setIsPrivate] = useState(false);
  const [nameError, setNameError] = useState(false);
  const [isUploading, setIsUploading] = useState(false);
  const [unreadChannels, setUnreadChannels] = useState<string[]>([]);
  const [isSidebarOpen, setIsSidebarOpen] = useState(false);
  const lastSeenMessageId = useRef<Record<string, number>>({});
  const scrollRef = useRef<HTMLDivElement>(null);
  const roles = rolesFromUserLike(user);
  const isAdminByName = useMemo(() => {
    if (!user?.fullName) return false;
    return AUTHORIZED_ADMINS.includes(user.fullName);
  }, [user]);
  const sortedUsers = useMemo(() => {
    return [...allUsers].sort((a, b) => {
      const nameA = a.name.split(' ').pop() || "";
      const nameB = b.name.split(' ').pop() || "";
      return nameA.localeCompare(nameB);
    });
  }, [allUsers]);
  useEffect(() => {
    if (user?.id) {
      const saved = localStorage.getItem(`lastSeen_${user.id}`);
      if (saved) {
        lastSeenMessageId.current = JSON.parse(saved);
      }
    }
  }, [user?.id]);
  const fetchChannels = async () => {
    try {
      const res = await fetch("/api/channels");
      const data = await res.json();
      const accessible = data.filter((c: Channel) => {
        if (c.type === "public") return true;
        if (c.type === "restricted" && c.roleRequired && roles.includes(c.roleRequired)) return true;
        if (c.type === "private" && c.members?.includes(user?.id || "")) return true;
        return false;
      });
      setChannels(accessible);
      if (accessible.length > 0 && activeChannel === "general") {
        const generalChan = accessible.find((c: Channel) => c.name.toLowerCase() === "general");
        if (generalChan) {
          setActiveChannel(generalChan.id);
        } else {
          setActiveChannel(accessible[0].id);
        }
      }
    } catch (err) {
      console.error("Erreur channels:", err);
    }
  };
  const fetchProfs = async () => {
    try {
      const res = await fetch("/api/channels/users/list");
      const data = await res.json();
      setAllUsers(data.filter((p: ClerkUser) => p.id !== user?.id));
    } catch (err) {
      console.error("Erreur liste profs:", err);
    }
  };
  const fetchMessages = async (isInitialLoad = false) => {
    if (isInitialLoad) setLoading(true);
    try {
      const res = await fetch("/api/channels/messages");
      const allMessages: Message[] = await res.json();
      const filtered = allMessages.filter((m) => m.channel === activeChannel);
      setMessages(filtered);
      const newUnreads: string[] = [];
      channels.forEach(chan => {
        const chanMessages = allMessages.filter(m => m.channel === chan.id);
        const lastMsgInChan = chanMessages[chanMessages.length - 1];
        if (chan.id === activeChannel) {
          if (lastMsgInChan) {
            lastSeenMessageId.current[chan.id] = lastMsgInChan.id;
            localStorage.setItem(`lastSeen_${user?.id}`, JSON.stringify(lastSeenMessageId.current));
          }
          return;
        } 
        const lastSeenId = lastSeenMessageId.current[chan.id] || 0;
        if (lastMsgInChan && lastMsgInChan.id > lastSeenId) {
          newUnreads.push(chan.id);
        }
      });
      setUnreadChannels(newUnreads);
    } catch (err) {
      console.error("Erreur messages:", err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    if (isLoaded && user) {
      fetchChannels();
      fetchProfs();
    }
  }, [isLoaded, user?.id]);

  useEffect(() => {
    if (isLoaded && activeChannel && channels.length > 0) {
      fetchMessages(true);
      const interval = setInterval(() => fetchMessages(false), 5000);
      return () => clearInterval(interval);
    }
  }, [activeChannel, isLoaded, channels.length]);

  useEffect(() => {
    if (scrollRef.current && !loading) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [messages, loading]);

  const handleSend = async (anonymous: boolean, overrideContent?: string) => {
    const contentToSend = overrideContent || text;
    if (!contentToSend.trim()) return;
    const res = await fetch("/api/channels/messages", {
      method: "POST",
      body: JSON.stringify({
        content: contentToSend,
        channel: activeChannel,
        isAnonymous: anonymous
      }),
    });
    if (res.ok) {
      if (!overrideContent) setText("");
      fetchMessages(false);
    }
  };

  const handleDeleteMessage = async (messageId: number) => {
    if (!confirm("Supprimer ce message ?")) return;
    try {
      const res = await fetch(`/api/channels/messages?id=${messageId}`, {
        method: "DELETE",
      });
      if (res.ok) fetchMessages(false);
    } catch (err) {
      console.error("Erreur suppression:", err);
    }
  };

  const handleDeleteChannel = async () => {
    if (!currentChannel) return;
    const firstConfirm = confirm(`Supprimer #${currentChannel.name} ?`);
    if (firstConfirm && prompt("Ecrire SUPPRIMER :") === "SUPPRIMER") {
      try {
        const res = await fetch(`/api/channels?id=${currentChannel.id}`, { method: "DELETE" });
        if (res.ok) { setActiveChannel("general"); fetchChannels(); }
      } catch (err) { console.error(err); }
    }
  };

  const handleUpdateMembers = async (newMembers: string[]) => {
    try {
      const res = await fetch("/api/channels", {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          channelId: activeChannel,
          members: newMembers 
        })
      });
      if (res.ok) {
        fetchChannels();
      }
    } catch (err) {
      console.error("Erreur mise à jour membres:", err);
    }
  };

  const handleFileUpload = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    setIsUploading(true);
    const formData = new FormData();
    formData.append("file", file);
    try {
      const res = await fetch("/api/channels/upload", { method: "POST", body: formData });
      const data = await res.json();
      if (data.url) {
        const content = file.type.startsWith("image/") ? `[IMG]${data.url}` : `[DOC]${data.name}|${data.url}`;
        await handleSend(false, content);
      }
    } catch (err) { 
      console.log(err)
      alert("Erreur envoi"); 
    } finally { setIsUploading(false); }
  };

  const toggleMemberCreation = (id: string) => { setSelectedMembers(prev => prev.includes(id) ? prev.filter(m => m !== id) : [...prev, id])};
  
  const toggleMemberUpdate = (id: string) => {
    if (!currentChannel) return;
    const currentList = currentChannel.members || [];
    const newList = currentList.includes(id)  ? currentList.filter(m => m !== id)  : [...currentList, id];
    if (currentChannel.creatorId && !newList.includes(currentChannel.creatorId)) { newList.push(currentChannel.creatorId);}
    handleUpdateMembers(newList);
  };

  const handleCreateChannel = async () => {
    setNameError(false);
    if (!newChanName.trim()) { setNameError(true); return; }
    try {
      const res = await fetch("/api/channels", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ 
          name: newChanName.trim(), 
          type: isPrivate ? "private" : "public",
          members: isPrivate ? [...selectedMembers, user?.id] : [],
          creatorId: user?.id 
        })
      });
      if (res.ok) {
        setNewChanName(""); setSelectedMembers([]); setIsPrivate(false);
        setShowModal(false); fetchChannels();
      }
    } catch (err) { console.error(err); }
  };

  if (!isLoaded) {
    return (
      <DashboardThemeRoot>
        <div className={`p-10 text-center text-sm ${dash.textMid}`}>Chargement...</div>
      </DashboardThemeRoot>
    );
  }
  const currentChannel = channels.find(c => c.id === activeChannel);
  const isCreator = currentChannel?.creatorId === user?.id;

  return (
    <DashboardThemeRoot>
    <main className="relative flex h-screen gap-4 overflow-hidden p-2 md:p-4">
      
      <div
        data-tour="channels-list"
        className={`
        ${isSidebarOpen ? "translate-x-0" : "-translate-x-[110%]"} 
        md:translate-x-0 fixed md:static inset-y-4 left-4 z-40
        w-64 rounded-[1.5rem] border border-white/55 bg-white/50 shadow-[0_20px_50px_-32px_rgba(15,23,42,0.45)] backdrop-blur-2xl md:shadow-sm 
        flex flex-col overflow-hidden transition-transform duration-300 ease-in-out
        md:flex
      `}>
        <div className="p-6 border-b flex justify-between items-center">
          <h2 className="font-bold text-gray-800">Salons</h2>
          <button 
            onClick={() => setShowModal(true)}
            className="w-8 h-8 flex items-center justify-center bg-blue-50 text-blue-600 rounded-xl hover:bg-blue-100 transition-all"
          >
            ＋
          </button>
        </div>
        <div className="flex-1 overflow-y-auto p-4 space-y-2">
          {channels.map((c) => (
            <button
              key={c.id}
              onClick={() => {
                setActiveChannel(c.id);
                setUnreadChannels(prev => prev.filter(id => id !== c.id));
                setIsSidebarOpen(false);
              }}
              className={`w-full text-left px-4 py-3 rounded-2xl text-sm font-medium transition-all flex items-center justify-between ${
                activeChannel === c.id 
                ? `${dash.bgPrimary} text-white shadow-md` 
                : "text-gray-600 hover:bg-gray-50"
              }`}
            >
              <span className="truncate">
                {c.type === 'private' ? '🔒' : '#'} {c.name}
              </span>
              {unreadChannels.includes(c.id) && (
                <span className="w-2 h-2 bg-red-500 rounded-full shadow-sm animate-pulse"></span>
              )}
            </button>
          ))}
        </div>
      </div>
      {isSidebarOpen && (
        <div  className="fixed inset-0 bg-black/20 z-30 md:hidden"  onClick={() => setIsSidebarOpen(false)}/>
      )}

      <div className="relative flex flex-1 flex-col overflow-hidden rounded-[1.5rem] border border-white/55 bg-white/50 shadow-[0_20px_50px_-32px_rgba(15,23,42,0.45)] backdrop-blur-2xl">
        <div className="p-4 md:p-5 border-b bg-white flex items-center justify-between">
          <div className="flex items-center gap-3">
            <button 
              onClick={() => setIsSidebarOpen(true)}
              className="md:hidden w-10 h-10 flex items-center justify-center bg-blue-50 text-blue-600 rounded-xl hover:bg-blue-100 transition-all text-xl font-bold"
            >
              »
            </button>
            <div>
              <h1 className="font-bold text-gray-800 text-base md:text-lg truncate max-w-[150px] md:max-w-none">
                # {currentChannel?.name || activeChannel}
              </h1>
              <p className="text-[10px] md:text-xs text-gray-400">Discussion entre collègues</p>
            </div>
          </div>
          <div className="flex items-center gap-2">
            {isCreator && activeChannel !== "general" && (
              <>
                <button 
                  onClick={() => setShowMembersModal(true)}
                  className="p-2 text-gray-400 hover:text-blue-500 hover:bg-blue-50 rounded-xl transition-all"
                  title="Gérer les membres"
                >
                  👥
                </button>
                <button 
                  onClick={handleDeleteChannel}
                  className="p-2 text-gray-400 hover:text-red-500 hover:bg-red-50 rounded-xl transition-all"
                  title="Supprimer le salon"
                >
                  🗑️
                </button>
              </>
            )}
          </div>
        </div>

        <div ref={scrollRef} data-tour="channels-messages" className="flex-1 overflow-y-auto p-4 md:p-6 space-y-6 bg-gray-50/30">
          {loading ? (
            <div className="space-y-6">
              {[1, 2, 3].map((i) => (
                <div key={i} className={`flex ${i % 2 === 0 ? "justify-end" : "justify-start"} animate-pulse`}>
                  <div className={`w-2/3 space-y-2 ${i % 2 === 0 ? "items-end" : "items-start"} flex flex-col`}>
                    <div className="h-3 w-24 bg-gray-200 rounded-full"></div>
                    <div className={`h-12 w-full bg-gray-200 rounded-2xl ${i % 2 === 0 ? "rounded-tr-none" : "rounded-tl-none"}`}></div>
                  </div>
                </div>
              ))}
            </div>
          ) : messages.length === 0 ? (
            <div className="h-full flex flex-col items-center justify-center text-gray-400 space-y-2">
              <span className="text-4xl">💬</span>
              <p className="text-sm italic">Aucun message ici.</p>
            </div>
          ) : (
            messages.map((m) => {
              const isMe = m.authorId === user?.id;
              const canDelete = isMe || isAdminByName;
              return (
                <div key={m.id} className={`flex ${isMe ? "justify-end" : "justify-start"} group`}>
                  <div className={`flex flex-col max-w-[85%] md:max-w-[70%] ${isMe ? "items-end" : "items-start"}`}>
                    <div className="flex items-center gap-2 mb-1 px-2">
                      {!isMe && canDelete && (
                         <button onClick={() => handleDeleteMessage(m.id)} className="opacity-0 group-hover:opacity-100 text-red-400 hover:text-red-600 text-[10px]">Supprimer</button>
                      )}
                      <span className="text-[10px] text-gray-400 truncate">
                        {m.authorName} • {new Date(m.createdAt).toLocaleTimeString([], {hour: '2-digit', minute:'2-digit'})}
                      </span>
                      {isMe && (
                         <button onClick={() => handleDeleteMessage(m.id)} className="opacity-0 group-hover:opacity-100 text-red-400 hover:text-red-600 text-[10px]">Supprimer</button>
                      )}
                    </div>
                    <div className={`px-4 py-3 rounded-2xl text-sm shadow-sm break-words w-full ${
                      isMe 
                      ? `${dash.bgPrimary} rounded-tr-none text-white` 
                      : "bg-white text-gray-800 border border-gray-100 rounded-tl-none"
                    }`}>
                      {m.content.startsWith("[IMG]") ? (
                        <Image 
                          src={m.content.replace("[IMG]", "")} 
                          className="max-w-full rounded-lg cursor-pointer" 
                          alt="Partage"
                          width={200}
                          height={200}
                          onClick={() => window.open(m.content.replace("[IMG]", ""), "_blank")}
                        />
                      ) : m.content.startsWith("[DOC]") ? (
                        <a 
                          href={m.content.split("|")[1]} 
                          target="_blank" 
                          className={`flex items-center gap-2 p-1 font-medium underline ${isMe ? "text-blue-100" : "text-blue-600"}`}
                        >
                          📄 {m.content.split("|")[0].replace("[DOC]", "")}
                        </a>
                      ) : (
                        m.content
                      )}
                    </div>
                  </div>
                </div>
              );
            })
          )}
        </div>
        <div data-tour="channels-compose" className="p-3 md:p-4 bg-white border-t">
          <div className={`rounded-2xl p-2 ${dash.bgSoft50}`}>
            <textarea
              value={text}
              onChange={(e) => setText(e.target.value)}
              className="w-full bg-transparent border-none focus:ring-0 text-sm p-3 resize-none h-16 md:h-20 text-gray-800"
              placeholder={isUploading ? "Envoi..." : `Message dans #${currentChannel?.name || activeChannel}...`}
              disabled={isUploading}
            />
            <div className="flex justify-between items-center p-1 md:p-2">
              <label className="cursor-pointer p-2 hover:bg-gray-200 rounded-xl">
                <input type="file" className="hidden" onChange={handleFileUpload} disabled={isUploading} />
                <span className="text-xl">📎</span>
              </label>
              <div className="flex gap-1 md:gap-2">
                <button 
                  onClick={() => handleSend(true)}
                  className="px-3 md:px-4 py-2 text-[10px] md:text-xs font-semibold text-gray-500 hover:bg-gray-200 rounded-xl"
                  disabled={isUploading}
                >
                  🕵️ Anonyme
                </button>
                <button 
                  onClick={() => handleSend(false)}
                  className={`px-4 py-2 text-[10px] font-semibold text-white shadow-lg md:px-6 md:text-xs rounded-xl ${dash.bgPrimary}`}
                  disabled={isUploading}
                >
                  Envoyer
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
      {showModal && (
        <div className="fixed inset-0 bg-black/40 backdrop-blur-sm flex items-center justify-center z-50 p-4">
          <div className="bg-white p-6 md:p-8 rounded-3xl shadow-2xl w-full max-w-[450px] max-h-[90vh] flex flex-col border border-gray-100">
            <h3 className="text-xl font-bold text-gray-800 mb-4">Nouveau salon</h3>
            <div className="mb-4">
              <input 
                autoFocus
                className={`w-full border p-4 rounded-2xl outline-none transition-all ${nameError ? 'border-red-500 bg-red-50' : 'border-gray-200 bg-gray-50 focus:ring-2 focus:ring-blue-500'}`} 
                placeholder="Nom du salon..." 
                value={newChanName}
                onChange={(e) => { setNewChanName(e.target.value); if(nameError) setNameError(false); }}
              />
              {nameError && <p className="text-red-500 text-xs mt-1 ml-2">Le nom est obligatoire</p>}
            </div>
            <div className="flex items-center gap-2 mb-4 px-2">
              <input type="checkbox" id="private" checked={isPrivate} onChange={(e) => setIsPrivate(e.target.checked)} className="w-4 h-4 rounded text-blue-600" />
              <label htmlFor="private" className="text-sm font-medium text-gray-700 select-none">Salon privé (Membres sélectionnés)</label>
            </div>
            {isPrivate && (
              <div className="flex-1 overflow-y-auto mb-6 bg-gray-50 rounded-2xl p-4 border border-gray-100">
                <p className="text-[10px] font-bold text-gray-400 uppercase mb-3">Sélectionner les utilisateurs (Tri A-Z)</p>
                <div className="space-y-2">
                  {sortedUsers.map(user => (
                    <div 
                      key={user.id} 
                      onClick={() => toggleMemberCreation(user.id)}
                      className={`flex items-center gap-3 p-2 rounded-xl cursor-pointer transition-all ${selectedMembers.includes(user.id) ? 'bg-blue-100' : 'hover:bg-white'}`}
                    >
                      <Image src={user.avatar} className="w-8 h-8 rounded-full border border-gray-200" width={100} height={100} alt="" />
                      <span className="text-sm flex-1 truncate">{user.name}</span>
                      <div className={`flex h-5 w-5 items-center justify-center rounded-full border ${selectedMembers.includes(user.id) ? `${dash.bgPrimary} border-[var(--dash-primary)]` : "border-gray-300"}`}>
                        {selectedMembers.includes(user.id) && <span className="text-white text-[10px]">✓</span>}
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            )}
            <div className="flex gap-3 mt-auto">
              <button onClick={() => { setShowModal(false); setNameError(false); }} className="flex-1 py-3 text-gray-500 font-semibold hover:bg-gray-100 rounded-2xl transition-all">Annuler</button>
              <button onClick={handleCreateChannel} className={`flex-1 rounded-2xl py-3 font-semibold text-white ${dash.bgPrimary}`}>Créer</button>
            </div>
          </div>
        </div>
      )}
      {showMembersModal && currentChannel && (
        <div className="fixed inset-0 bg-black/40 backdrop-blur-sm flex items-center justify-center z-50 p-4">
          <div className="bg-white p-6 md:p-8 rounded-3xl shadow-2xl w-full max-w-[450px] max-h-[90vh] flex flex-col border border-gray-100">
            <div className="flex justify-between items-center mb-6">
              <h3 className="text-xl font-bold text-gray-800">Membres du salon</h3>
              <button onClick={() => setShowMembersModal(false)} className="text-gray-400 hover:text-gray-600">✕</button>
            </div>
            <div className="flex-1 overflow-y-auto mb-6 space-y-2 pr-2">
              <p className="text-[10px] font-bold text-gray-400 uppercase mb-3">Gérer l&apos;accès (Tri par nom A-Z)</p>
              {sortedUsers.map(user => {
                const isMember = currentChannel.members?.includes(user.id);
                return (
                  <div 
                    key={user.id} 
                    onClick={() => toggleMemberUpdate(user.id)}
                    className={`flex items-center gap-3 p-3 rounded-2xl cursor-pointer transition-all border ${isMember ? 'bg-blue-50 border-blue-100' : 'bg-gray-50 border-transparent hover:bg-white hover:border-gray-200'}`}
                  >
                    <Image src={user.avatar} width={100} height={100} className="w-9 h-9 rounded-full border border-gray-200" alt="" />
                    <div className="flex-1 min-w-0">
                      <p className="text-sm font-semibold text-gray-800 truncate">{user.name}</p>
                      <p className="text-[10px] text-gray-500">{isMember ? "A accès au salon" : "Pas d'accès"}</p>
                    </div>
                    <div className={`relative h-6 w-10 rounded-full transition-colors duration-200 ${isMember ? dash.bgPrimary : "bg-gray-300"}`}>
                      <div className={`absolute top-1 bg-white w-4 h-4 rounded-full transition-all duration-200 ${isMember ? 'left-5' : 'left-1'}`} />
                    </div>
                  </div>
                );
              })}
            </div>
            <button 
              onClick={() => setShowMembersModal(false)}
              className="w-full py-4 bg-gray-900 text-white font-bold rounded-2xl hover:bg-black transition-all shadow-lg"
            >
              Terminer
            </button>
          </div>
        </div>
      )}
      <div className="mt-12 border-t border-white/60 pt-4">
        <ReplayModuleTourButton moduleId="channels" />
      </div>
    </main>
    </DashboardThemeRoot>
  );
}