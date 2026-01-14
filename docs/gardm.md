# GARDM

the next step in the gar desktop experience is the gar desktop manager. it should be oriented towards integration with other gar products. 
it should set the same background as will be set by garbg on launching gar so that the only thing that changes during the transition is the animation to load in

it should have users centered, and all the standard desktop manger buttons.

it should have a sleek interface like my current sddm approach. background should be blurred. username/pass should be  centered. 

first stage is planning. we need to reference existing foss greeters/desktop managers to see what ours needs to do, otherwise we're lost. 
We'll make a checklist of targets to hit and generate sprint files in the .gitignored directory docs/sprints/. each sprint file should have a list of targets and pitfalls to avoid

gardm should do everything that a modern greeter does, and look pretty while doing it. it should be gar-first, that is oriented towards the gar desktop environment and window manager, but it should be compatible with all linux machines that support x11. 
we're going with x11 because of nvidia issues on wayland, but we can still make a pretty greeter in xorgland.