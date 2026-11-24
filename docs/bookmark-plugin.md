# The bookmark plugin

The bookmark plugin gives Organizr a page of links. Each link is a colored tile, and the tiles stand in categories.

Bookmarks are apart from the tabs of the menu. A tab opens an application inside Organizr. A bookmark opens an address in a new window.

## Set it up

1. Log in as an Admin.
2. Open **Settings**, select the tab **Plugins**, and select **Inactive**.
3. Point at the card **Bookmark**, and select the button with the plug.
4. Select **Active**. Point at the card **Bookmark**, and select the button with the panel. The window **Bookmark Settings** opens. The tab **Bookmarks** shows the editor of the bookmarks.
5. Enter the name of a category, and select **Add a category**.
6. Select **Add a bookmark**, and fill in the form.
7. Select **Make the tab Bookmarks**. The menu then has a tab that shows the page.

| Field of a bookmark | Meaning |
|---|---|
| Name | The text on the tile. Two bookmarks cannot have the same name |
| Address | The address that the tile opens |
| Picture | A picture or an icon, as for a tab |
| Category | The heading that the tile stands under |
| For the group, and groups with more access | The group with the least access that sees the bookmark |
| Color of the tile | The page shows the tile in shades of this color |
| Color of the name | The color of the text on the tile |
| Show the bookmark on the page | A bookmark that is switched off stays in the editor only |

## On the homepage

The homepage can show the bookmarks too: switch on the homepage item **Bookmarks**. See [Bookmarks](homepage.md#bookmarks) on the homepage page.

## Who sees what

The page is for the people who see its tab in the menu. You set that on the tab **Tabs** of the settings, as for each tab.

On the page, a person sees the bookmarks of the own group and of groups with less access. A category without a bookmark for that person is not shown.

## Categories

One category is always the default. A new bookmark joins it when the form names no other category. The first category is the default. When you delete the default category, the category with the lowest number becomes the default.

When you delete a category, its bookmarks stay. The page does not show them until you give them another category.

## The routes

The plugin has these routes under `/api/v2/plugins/bookmark`. All but the first need an Admin or a Co-Admin.

| Route | Purpose |
|---|---|
| `GET page` | The bookmarks that the caller sees, in their categories |
| `GET tabs`, `GET categories` | Every bookmark, every category and the groups |
| `GET tabs/{id}` | One bookmark, for a caller of its group |
| `POST tabs`, `PUT tabs/{id}`, `DELETE tabs/{id}` | Add, change and delete a bookmark |
| `PUT tabs` | New places for bookmarks: a list of `id` and `order` |
| `POST categories`, `PUT categories/{id}`, `DELETE categories/{id}` | Add, change and delete a category |
| `PUT categories` | New places for categories |
| `GET setup/tab` | Make the tab "Bookmarks" of the menu |
| `GET setup/category` | Make the category "Unsorted" when there is no category |

## Bookmarks of PHP Organizr

An import brings the bookmarks and their categories with their ids. [Importing](importing.md) says what the import repairs on the way, for example two bookmarks with the same name.

After the import, the tab of the menu that showed the page in PHP Organizr shows the page here too.
